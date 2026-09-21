pub mod projections;

use std::{
    cell::LazyCell,
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Write as _},
    path::{Path, PathBuf},
};

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
use zstd::Encoder;

use crate::{
    enums::{LabelType, Metric},
    solver::projections::{DiagonalsPruneTarget4x4, SplitFringePruneTarget4x4},
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

fn write_compressed_pdb(pdb_file_path: &Path, bytes: &[u8]) {
    let file = File::create(pdb_file_path).unwrap();
    let writer = BufWriter::new(file);

    let mut encoder = Encoder::new(writer, 0).unwrap();
    encoder.include_checksum(true).unwrap();
    encoder.include_contentsize(true).unwrap();
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap();
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

                    let pdb = if let Ok(file) = File::open(&pdb_file_path) {
                        let reader = BufReader::new(file);
                        let bytes = zstd::decode_all(reader).unwrap().into_boxed_slice();

                        // SAFETY: this computes a checksum to verify correctness, which is
                        // good enough here.
                        unsafe { PdbTy::try_from_bytes(bytes) }.unwrap()
                    } else {
                        let pdb = PdbTy::default();

                        let bytes = pdb.as_ref();
                        write_compressed_pdb(&pdb_file_path, bytes);

                        pdb
                    };

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
            || {
                let size = Size::new(4, 4).unwrap();
                let pdb_file_path = pdb_file_path(size, LabelType::RowGrids, Metric::Mtm);

                File::open(&pdb_file_path).map_or_else(
                    |_| {
                        let solver = Solver4x4Mtm::default();

                        let bytes = solver.pdb().as_ref();
                        write_compressed_pdb(&pdb_file_path, bytes);

                        solver
                    },
                    |file| {
                        let reader = BufReader::new(file);
                        let bytes = zstd::decode_all(reader).unwrap().into_boxed_slice();

                        // SAFETY: this computes a checksum to verify correctness, which is
                        // good enough here.
                        unsafe { Solver4x4Mtm::try_with_pdb_bytes(bytes) }.unwrap()
                    },
                )
            },
        );

        // Labels

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

                        // SAFETY: It's possible for the user to manually swap out a pdb file with
                        // a bad file, but realistically they probably won't. We store them with a
                        // checksum to guard against file corruption.
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
                        write_compressed_pdb(&pdb_file_path, bytes);

                        solver
                    }
                };

                this.register(size, target, metric, solver);
            }};
            ($w:literal, $h:literal, $target:tt, $prune_target:tt) => {
                register_projection_solver!($w, $h, $target, $prune_target, Stm);
                register_projection_solver!($w, $h, $target, $prune_target, Mtm);
            };
            ($w:literal, $h:literal, $target:tt) => {
                register_projection_solver!($w, $h, $target, $target);
            };
        }

        // Rows

        register_projection_solver!(2, 2, Rows);
        register_projection_solver!(2, 3, Rows);
        register_projection_solver!(2, 4, Rows);
        register_projection_solver!(2, 5, Rows);
        register_projection_solver!(2, 6, Rows);

        register_projection_solver!(3, 2, Rows);
        register_projection_solver!(3, 3, Rows);
        register_projection_solver!(3, 4, Rows);
        register_projection_solver!(3, 5, Rows);

        register_projection_solver!(4, 2, Rows);
        register_projection_solver!(4, 3, Rows);
        register_projection_solver!(4, 4, Rows);

        register_projection_solver!(5, 2, Rows);
        register_projection_solver!(5, 3, Rows);

        register_projection_solver!(6, 2, Rows);
        register_projection_solver!(7, 2, Rows);
        register_projection_solver!(8, 2, Rows);
        register_projection_solver!(9, 2, Rows);
        register_projection_solver!(10, 2, Rows);
        register_projection_solver!(11, 2, Rows);
        register_projection_solver!(12, 2, Rows);

        // Fringe
        // TODO: make small x big transpose to big x small

        register_projection_solver!(2, 2, Fringe);
        register_projection_solver!(2, 3, Fringe);
        register_projection_solver!(2, 4, Fringe);
        register_projection_solver!(2, 5, Fringe);
        register_projection_solver!(2, 6, Fringe);
        register_projection_solver!(2, 7, Fringe);
        register_projection_solver!(2, 8, Fringe);
        register_projection_solver!(2, 9, Fringe);
        register_projection_solver!(2, 10, Fringe);
        register_projection_solver!(2, 11, Fringe);
        register_projection_solver!(2, 12, Fringe);
        register_projection_solver!(2, 13, Fringe);
        register_projection_solver!(2, 14, Fringe);

        register_projection_solver!(3, 2, Fringe);
        register_projection_solver!(3, 3, Fringe);
        register_projection_solver!(3, 4, Fringe);
        register_projection_solver!(3, 5, Fringe);
        register_projection_solver!(3, 6, Fringe);

        register_projection_solver!(4, 2, Fringe);
        register_projection_solver!(4, 3, Fringe);
        register_projection_solver!(4, 4, Fringe);

        register_projection_solver!(5, 2, Fringe);
        register_projection_solver!(6, 2, Fringe);
        register_projection_solver!(7, 2, Fringe);
        register_projection_solver!(8, 2, Fringe);
        register_projection_solver!(9, 2, Fringe);
        register_projection_solver!(10, 2, Fringe);
        register_projection_solver!(11, 2, Fringe);
        register_projection_solver!(12, 2, Fringe);
        register_projection_solver!(13, 2, Fringe);
        register_projection_solver!(14, 2, Fringe);

        // Square fringe
        // NxN is equivalent to fringe

        register_projection_solver!(2, 3, SquareFringe);
        register_projection_solver!(2, 4, SquareFringe);
        register_projection_solver!(2, 5, SquareFringe);
        register_projection_solver!(2, 6, SquareFringe);
        register_projection_solver!(2, 7, SquareFringe);

        register_projection_solver!(3, 2, SquareFringe);
        register_projection_solver!(3, 4, SquareFringe);

        register_projection_solver!(4, 2, SquareFringe);
        register_projection_solver!(4, 3, SquareFringe);

        register_projection_solver!(5, 2, SquareFringe);
        register_projection_solver!(5, 3, SquareFringe);

        register_projection_solver!(6, 2, SquareFringe);
        register_projection_solver!(7, 2, SquareFringe);

        // Split fringe

        register_projection_solver!(2, 2, SplitFringe);
        register_projection_solver!(2, 3, SplitFringe);
        register_projection_solver!(2, 4, SplitFringe);
        register_projection_solver!(2, 5, SplitFringe);
        register_projection_solver!(2, 6, SplitFringe);
        register_projection_solver!(2, 7, SplitFringe);
        register_projection_solver!(2, 8, SplitFringe);
        register_projection_solver!(2, 9, SplitFringe);
        register_projection_solver!(2, 10, SplitFringe);

        register_projection_solver!(3, 2, SplitFringe);
        register_projection_solver!(3, 3, SplitFringe);
        register_projection_solver!(3, 4, SplitFringe);
        register_projection_solver!(3, 5, SplitFringe);

        register_projection_solver!(4, 2, SplitFringe);
        register_projection_solver!(4, 3, SplitFringe);
        register_projection_solver!(4, 4, SplitFringe, SplitFringePruneTarget4x4);

        register_projection_solver!(5, 2, SplitFringe);
        register_projection_solver!(6, 2, SplitFringe);
        register_projection_solver!(7, 2, SplitFringe);
        register_projection_solver!(8, 2, SplitFringe);
        register_projection_solver!(9, 2, SplitFringe);
        register_projection_solver!(10, 2, SplitFringe);

        // Split square fringe
        // 2xN is equivalent to rows
        // NxN is equivalent to split fringe

        register_projection_solver!(3, 2, SplitSquareFringe);
        register_projection_solver!(3, 4, SplitSquareFringe);

        register_projection_solver!(4, 2, SplitSquareFringe);
        register_projection_solver!(4, 3, SplitSquareFringe);

        register_projection_solver!(5, 2, SplitSquareFringe);
        register_projection_solver!(6, 2, SplitSquareFringe);
        register_projection_solver!(7, 2, SplitSquareFringe);

        // Diagonals
        // TODO: make small x big transpose to big x small

        register_projection_solver!(2, 2, Diagonals);
        register_projection_solver!(2, 3, Diagonals);
        register_projection_solver!(2, 4, Diagonals);
        register_projection_solver!(2, 5, Diagonals);
        register_projection_solver!(2, 6, Diagonals);

        register_projection_solver!(3, 2, Diagonals);
        register_projection_solver!(3, 3, Diagonals);
        register_projection_solver!(3, 4, Diagonals);

        register_projection_solver!(4, 2, Diagonals);
        register_projection_solver!(4, 3, Diagonals);
        register_projection_solver!(4, 4, Diagonals, DiagonalsPruneTarget4x4);

        register_projection_solver!(4, 2, Diagonals);
        register_projection_solver!(5, 2, Diagonals);
        register_projection_solver!(6, 2, Diagonals);
        register_projection_solver!(7, 2, Diagonals);

        // Checkerboard
        // TODO: make small x big transpose to big x small

        register_projection_solver!(2, 2, Checkerboard);
        register_projection_solver!(2, 3, Checkerboard);
        register_projection_solver!(2, 4, Checkerboard);
        register_projection_solver!(2, 5, Checkerboard);
        register_projection_solver!(2, 6, Checkerboard);
        register_projection_solver!(2, 7, Checkerboard);
        register_projection_solver!(2, 8, Checkerboard);
        register_projection_solver!(2, 9, Checkerboard);
        register_projection_solver!(2, 10, Checkerboard);
        register_projection_solver!(2, 11, Checkerboard);
        register_projection_solver!(2, 12, Checkerboard);
        register_projection_solver!(2, 13, Checkerboard);
        register_projection_solver!(2, 14, Checkerboard);

        register_projection_solver!(3, 2, Checkerboard);
        register_projection_solver!(3, 3, Checkerboard);
        register_projection_solver!(3, 4, Checkerboard);
        register_projection_solver!(3, 5, Checkerboard);
        register_projection_solver!(3, 6, Checkerboard);
        register_projection_solver!(3, 7, Checkerboard);
        register_projection_solver!(3, 8, Checkerboard);
        register_projection_solver!(3, 9, Checkerboard);

        register_projection_solver!(4, 2, Checkerboard);
        register_projection_solver!(4, 3, Checkerboard);
        register_projection_solver!(4, 4, Checkerboard);
        register_projection_solver!(4, 5, Checkerboard);
        register_projection_solver!(4, 6, Checkerboard);
        register_projection_solver!(4, 7, Checkerboard);

        register_projection_solver!(5, 2, Checkerboard);
        register_projection_solver!(5, 3, Checkerboard);
        register_projection_solver!(5, 4, Checkerboard);
        register_projection_solver!(5, 5, Checkerboard);

        register_projection_solver!(6, 2, Checkerboard);
        register_projection_solver!(6, 3, Checkerboard);
        register_projection_solver!(6, 4, Checkerboard);

        register_projection_solver!(7, 2, Checkerboard);
        register_projection_solver!(7, 3, Checkerboard);
        register_projection_solver!(7, 4, Checkerboard);

        register_projection_solver!(8, 2, Checkerboard);
        register_projection_solver!(8, 3, Checkerboard);

        register_projection_solver!(9, 2, Checkerboard);
        register_projection_solver!(9, 3, Checkerboard);

        register_projection_solver!(10, 2, Checkerboard);
        register_projection_solver!(11, 2, Checkerboard);
        register_projection_solver!(12, 2, Checkerboard);
        register_projection_solver!(13, 2, Checkerboard);
        register_projection_solver!(14, 2, Checkerboard);

        // Remaps

        // Map square fringe to normal fringe on square puzzles
        this.register_remap(|key| {
            if !key.size.is_square() {
                return None;
            }

            let new_label = match key.label {
                LabelType::SquareFringe => LabelType::Fringe,
                LabelType::SplitSquareFringe => LabelType::SplitFringe,
                LabelType::Trivial
                | LabelType::RowGrids
                | LabelType::Rows
                | LabelType::Fringe
                | LabelType::SplitFringe
                | LabelType::Diagonals
                | LabelType::Checkerboard => return None,
            };

            Some(SolverKey {
                label: new_label,
                ..key
            })
        });

        // Map split square fringe 2xN to rows
        this.register_remap(|key| {
            (key.size.width() == 2 && key.label == LabelType::SplitSquareFringe).then_some(
                SolverKey {
                    label: LabelType::Rows,
                    ..key
                },
            )
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
