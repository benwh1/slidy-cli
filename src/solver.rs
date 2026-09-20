pub mod projections;

use std::{cell::LazyCell, collections::HashMap, fs::File, io::BufReader, path::PathBuf};

use directories::ProjectDirs;
use slidy::{
    algorithm::metric::{Mtm, Stm},
    puzzle::{
        label::label::{
            Checkerboard, Diagonals, Fringe, RowGrids, Rows, SplitFringe, SplitSquareFringe,
            SquareFringe, Trivial,
        },
        puzzle::Puzzle,
        size::Size,
        sliding_puzzle::SlidingPuzzle as _,
    },
    solver::{
        config::{PdbConfig, SolverConfig},
        generic_solver::GenericSolver,
        heuristic::{manhattan::ManhattanDistance, mtm::MtmHeuristic},
        projection::{pdb::Pdb as ProjectionPdb, solver::Solver as ProjectionSolver},
        solver::{Solver as SolverT, SolverError},
        Solver4x4Mtm, Solver4x4Stm,
    },
};

use crate::{
    enums::{LabelType, Metric},
    solver::projections::SplitFringePruneTarget4x4,
};

type BoxSolver = Box<dyn SolverT<Puzzle, Context = ()>>;
type BoxSolverInit = Box<dyn FnOnce() -> BoxSolver>;

type Remap = Box<dyn Fn(SolverKey) -> Option<SolverKey>>;

fn pdb_dir() -> PathBuf {
    let mut dir = ProjectDirs::from("", "", "slidy-cli")
        .unwrap()
        .cache_dir()
        .to_path_buf();
    dir.push("solver");
    dir.push("pdb");

    dir
}

fn pdb_file_path(size: Size, label: LabelType, metric: Metric) -> PathBuf {
    let path = format!(
        "{w}x{h}-{label}-{metric}.pdb.zst",
        w = size.width(),
        h = size.height(),
        label = label.pdb_file_name(),
        metric = metric.pdb_file_name(),
    );

    let mut file_path = pdb_dir();
    file_path.push(path);

    file_path
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SolverKey {
    size: Size,
    label: LabelType,
    metric: Metric,
}

pub struct Solver {
    solvers: HashMap<SolverKey, LazyCell<BoxSolver, BoxSolverInit>>,
    remaps: Vec<Remap>,
}

impl Solver {
    pub fn new() -> Self {
        let mut this = Self {
            solvers: HashMap::new(),
            remaps: Vec::new(),
        };

        std::fs::create_dir_all(pdb_dir()).unwrap();

        macro_rules! small {
            ($pdb_file_path:expr, $pdb_ty:ty, $solver_ty:ty) => {{
                let pdb_file_path = $pdb_file_path.clone();

                move || {
                    type PdbTy = $pdb_ty;
                    type SolverTy = $solver_ty;

                    let pdb = std::fs::read(&pdb_file_path).map_or_else(
                        |_| {
                            let pdb = PdbTy::default();
                            std::fs::write(&pdb_file_path, pdb.as_ref()).unwrap();
                            pdb
                        },
                        |bytes| {
                            // SAFETY: this computes a checksum to verify correctness, which is
                            // good enough here.
                            unsafe { PdbTy::try_from_bytes(bytes.into_boxed_slice()) }
                                .unwrap_or_else(|| {
                                    let pdb = PdbTy::default();
                                    std::fs::write(&pdb_file_path, pdb.as_ref()).unwrap();
                                    pdb
                                })
                        },
                    );

                    SolverTy::with_pdb(pdb)
                }
            }};
        }

        macro_rules! register_small_solver {
            ($max:literal, $min:literal) => {{
                use slidy::solver::small::{pdb::Pdb, solver::Solver};

                type PdbStm = Pdb<$max, $min, { $max * $min }, Stm>;
                type PdbMtm = Pdb<$max, $min, { $max * $min }, Mtm>;
                type SolverStm = Solver<$max, $min, { $max * $min }, Stm>;
                type SolverMtm = Solver<$max, $min, { $max * $min }, Mtm>;

                let max = $max;
                let min = $min;

                let size = Size::new(max, min).unwrap();

                let file_name_stm = pdb_file_path(size, LabelType::RowGrids, Metric::Stm);
                let file_name_mtm = pdb_file_path(size, LabelType::RowGrids, Metric::Mtm);

                this.register(
                    size,
                    LabelType::RowGrids,
                    Metric::Stm,
                    small!(file_name_stm, PdbStm, SolverStm),
                );
                this.register(
                    size,
                    LabelType::RowGrids,
                    Metric::Mtm,
                    small!(file_name_mtm, PdbMtm, SolverMtm),
                );

                if max != min {
                    let size = size.transpose();

                    this.register(
                        size,
                        LabelType::RowGrids,
                        Metric::Stm,
                        small!(file_name_stm, PdbStm, SolverStm),
                    );
                    this.register(
                        size,
                        LabelType::RowGrids,
                        Metric::Mtm,
                        small!(file_name_mtm, PdbMtm, SolverMtm),
                    );
                }
            }};
        }

        // Small puzzles

        register_small_solver!(3, 2);
        register_small_solver!(3, 3);
        register_small_solver!(4, 2);
        register_small_solver!(4, 3);
        register_small_solver!(5, 2);
        register_small_solver!(6, 2);

        // 4x4 STM and MTM

        this.register(
            Size::new(4, 4).unwrap(),
            LabelType::RowGrids,
            Metric::Stm,
            Solver4x4Stm::default,
        );

        this.register(
            Size::new(4, 4).unwrap(),
            LabelType::RowGrids,
            Metric::Mtm,
            {
                move || {
                    let size = Size::new(4, 4).unwrap();
                    let pdb_file_path = pdb_file_path(size, LabelType::RowGrids, Metric::Stm);

                    let make_solver = || {
                        let solver = Solver4x4Mtm::default();
                        let pdb = solver.pdb();
                        std::fs::write(&pdb_file_path, pdb.as_ref()).unwrap();
                        solver
                    };

                    std::fs::read(&pdb_file_path).map_or_else(
                        |_| make_solver(),
                        |bytes| {
                            // SAFETY: this computes a checksum to verify correctness, which is
                            // good enough here.
                            unsafe { Solver4x4Mtm::try_with_pdb_bytes(bytes.into_boxed_slice()) }
                                .unwrap_or_else(make_solver)
                        },
                    )
                }
            },
        );

        // 4x4 labels

        macro_rules! register_projection_solver {
            ($w:literal, $h:literal, $target:tt, $prune_target:tt, $metric:tt) => {{
                let w = $w;
                let h = $h;
                let target = LabelType::$target;
                let metric = Metric::$metric;

                let size = Size::new(w, h).unwrap();

                let solver = move || {
                    let builder = ProjectionSolver::builder()
                        .size(size)
                        .target($target)
                        .prune_target($prune_target)
                        .metric($metric);

                    let pdb_file_path = pdb_file_path(size, target, metric);

                    if let Ok(file) = File::open(&pdb_file_path) {
                        let reader = BufReader::new(file);
                        let bytes = zstd::decode_all(reader).unwrap().into_boxed_slice();
                        let pdb = unsafe { ProjectionPdb::from_bytes_unchecked(bytes) };

                        builder.pdb(pdb).build().unwrap()
                    } else {
                        let solver = builder
                            .pdb_config(PdbConfig {
                                end_of_iter_callback: Some(Box::new(|s| {
                                    println!("depth {} new {} total {}", s.depth, s.new, s.total);
                                })),
                            })
                            .build()
                            .unwrap();
                        let bytes = solver.pdb().as_ref();
                        let compressed = zstd::encode_all(bytes, 0).unwrap();

                        std::fs::write(&pdb_file_path, compressed).unwrap();

                        solver
                    }
                };

                this.register(size, target, metric, solver);
            }};
            ($w:literal, $h:literal, $target:tt, $prune_target:tt) => {
                register_projection_solver!($w, $h, $target, $prune_target, Stm);
                register_projection_solver!($w, $h, $target, $prune_target, Mtm);
            };
        }

        register_projection_solver!(4, 4, Rows, Rows);
        register_projection_solver!(4, 4, Fringe, Fringe);
        register_projection_solver!(4, 4, SplitFringe, SplitFringePruneTarget4x4);

        // Remaps

        // Map square fringe to normal fringe on square puzzles
        this.register_remap(|key| {
            if !key.size.is_square() {
                return None;
            }

            let new_label = match key.label {
                LabelType::SquareFringe => LabelType::Fringe,
                LabelType::SplitSquareFringe => LabelType::SplitFringe,
                _ => return None,
            };

            Some(SolverKey {
                label: new_label,
                ..key
            })
        });

        this
    }

    fn register<S, F>(&mut self, size: Size, label: LabelType, metric: Metric, solver: F)
    where
        S: SolverT<Puzzle, Context = ()> + 'static,
        F: FnOnce() -> S + 'static,
    {
        self.solvers.insert(
            SolverKey {
                size,
                label,
                metric,
            },
            LazyCell::new(Box::new(move || Box::new(solver()))),
        );
    }

    fn register_remap<F>(&mut self, remap: F)
    where
        F: Fn(SolverKey) -> Option<SolverKey> + 'static,
    {
        self.remaps.push(Box::new(remap));
    }

    fn remap_key(&self, mut key: SolverKey) -> SolverKey {
        while let Some(new_key) = self.remaps.iter().find_map(|remap| remap(key)) {
            key = new_key;
        }

        key
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SolverContext {
    pub metric: Metric,
    pub label: LabelType,
}

impl SolverT<Puzzle> for Solver {
    type Context = SolverContext;

    fn solve_with_config_and_context(
        &mut self,
        puzzle: &Puzzle,
        config: SolverConfig,
        context: &Self::Context,
    ) -> Result<(), SolverError> {
        let key = self.remap_key(SolverKey {
            size: puzzle.size(),
            label: context.label,
            metric: context.metric,
        });

        match self.solvers.get_mut(&key) {
            Some(s) => s.solve_with_config(puzzle, config),
            None => fallback_solver(SolverContext {
                metric: key.metric,
                label: key.label,
            })
            .solve_with_config(puzzle, config),
        }
    }
}

fn fallback_solver(context: SolverContext) -> Box<dyn SolverT<Puzzle, Context = ()>> {
    let SolverContext { metric, label } = context;

    match (metric, label) {
        (Metric::Stm, LabelType::Trivial) => {
            Box::new(GenericSolver::new(ManhattanDistance(Trivial), Trivial))
        }
        (Metric::Stm, LabelType::RowGrids) => {
            Box::new(GenericSolver::new(ManhattanDistance(RowGrids), RowGrids))
        }
        (Metric::Stm, LabelType::Rows) => {
            Box::new(GenericSolver::new(ManhattanDistance(Rows), Rows))
        }
        (Metric::Stm, LabelType::Fringe) => {
            Box::new(GenericSolver::new(ManhattanDistance(Fringe), Fringe))
        }
        (Metric::Stm, LabelType::SquareFringe) => Box::new(GenericSolver::new(
            ManhattanDistance(SquareFringe),
            SquareFringe,
        )),
        (Metric::Stm, LabelType::SplitFringe) => Box::new(GenericSolver::new(
            ManhattanDistance(SplitFringe),
            SplitFringe,
        )),
        (Metric::Stm, LabelType::SplitSquareFringe) => Box::new(GenericSolver::new(
            ManhattanDistance(SplitSquareFringe),
            SplitSquareFringe,
        )),
        (Metric::Stm, LabelType::Diagonals) => {
            Box::new(GenericSolver::new(ManhattanDistance(Diagonals), Diagonals))
        }
        (Metric::Stm, LabelType::Checkerboard) => Box::new(GenericSolver::new(
            ManhattanDistance(Checkerboard),
            Checkerboard,
        )),
        (Metric::Mtm, LabelType::Trivial) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(Trivial)),
            Trivial,
        )),
        (Metric::Mtm, LabelType::RowGrids) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(RowGrids)),
            RowGrids,
        )),
        (Metric::Mtm, LabelType::Rows) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(Rows)),
            Rows,
        )),
        (Metric::Mtm, LabelType::Fringe) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(Fringe)),
            Fringe,
        )),
        (Metric::Mtm, LabelType::SquareFringe) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(SquareFringe)),
            SquareFringe,
        )),
        (Metric::Mtm, LabelType::SplitFringe) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(SplitFringe)),
            SplitFringe,
        )),
        (Metric::Mtm, LabelType::SplitSquareFringe) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(SplitSquareFringe)),
            SplitSquareFringe,
        )),
        (Metric::Mtm, LabelType::Diagonals) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(Diagonals)),
            Diagonals,
        )),
        (Metric::Mtm, LabelType::Checkerboard) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(Checkerboard)),
            Checkerboard,
        )),
    }
}
