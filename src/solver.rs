use std::{cell::LazyCell, collections::HashMap};

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
        config::SolverConfig,
        generic_solver::GenericSolver,
        heuristic::{manhattan::ManhattanDistance, mtm::MtmHeuristic},
        solver::{Solver as SolverT, SolverError},
        Solver4x4Mtm, Solver4x4Stm,
    },
};

use crate::enums::{LabelType, Metric};

type BoxSolver = Box<dyn SolverT<Puzzle, Context = ()>>;
type BoxSolverInit = Box<dyn FnOnce() -> BoxSolver>;

#[derive(PartialEq, Eq, Hash)]
struct SolverKey {
    size: Size,
    metric: Metric,
    label: LabelType,
}

pub struct Solver {
    solvers: HashMap<SolverKey, LazyCell<BoxSolver, BoxSolverInit>>,
}

impl Solver {
    pub fn new() -> Self {
        let mut this = Self {
            solvers: HashMap::new(),
        };

        let mut pdb_cache_dir = ProjectDirs::from("", "", "slidy-cli")
            .unwrap()
            .cache_dir()
            .to_path_buf();
        pdb_cache_dir.push("solver");
        pdb_cache_dir.push("pdb");

        std::fs::create_dir_all(&pdb_cache_dir).unwrap();

        macro_rules! small {
            ($pdb_file:expr, $pdb_ty:ty, $solver_ty:ty) => {{
                let pdb_file = $pdb_file.clone();
                let pdb_cache_dir = pdb_cache_dir.clone();

                move || {
                    type PdbTy = $pdb_ty;
                    type SolverTy = $solver_ty;

                    let pdb_file = pdb_cache_dir.join(&pdb_file);

                    let pdb = std::fs::read(&pdb_file).map_or_else(
                        |_| {
                            let pdb = PdbTy::default();
                            std::fs::write(&pdb_file, pdb.as_ref()).unwrap();
                            pdb
                        },
                        |bytes| {
                            // SAFETY: this computes a checksum to verify correctness, which is
                            // good enough here.
                            unsafe { PdbTy::try_from_bytes(bytes.into_boxed_slice()) }
                                .unwrap_or_else(|| {
                                    let pdb = PdbTy::default();
                                    std::fs::write(&pdb_file, pdb.as_ref()).unwrap();
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

                let file_name_stm = format!("{max}x{min}-stm.bin");
                let file_name_mtm = format!("{max}x{min}-mtm.bin");

                this.register(
                    size,
                    Metric::Stm,
                    LabelType::RowGrids,
                    small!(file_name_stm, PdbStm, SolverStm),
                );
                this.register(
                    size,
                    Metric::Mtm,
                    LabelType::RowGrids,
                    small!(file_name_mtm, PdbMtm, SolverMtm),
                );

                if max != min {
                    let size = size.transpose();

                    this.register(
                        size,
                        Metric::Stm,
                        LabelType::RowGrids,
                        small!(file_name_stm, PdbStm, SolverStm),
                    );
                    this.register(
                        size,
                        Metric::Mtm,
                        LabelType::RowGrids,
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
            Metric::Stm,
            LabelType::RowGrids,
            Solver4x4Stm::default,
        );

        this.register(
            Size::new(4, 4).unwrap(),
            Metric::Mtm,
            LabelType::RowGrids,
            {
                let pdb_cache_dir = pdb_cache_dir.clone();

                move || {
                    let pdb_file = pdb_cache_dir.join("4x4-mtm.bin");

                    let make_solver = || {
                        let solver = Solver4x4Mtm::default();
                        let pdb = solver.pdb();
                        std::fs::write(&pdb_file, pdb.as_ref()).unwrap();
                        solver
                    };

                    std::fs::read(&pdb_file).map_or_else(
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

        this
    }

    fn register<S, F>(&mut self, size: Size, metric: Metric, label: LabelType, solver: F)
    where
        S: SolverT<Puzzle, Context = ()> + 'static,
        F: FnOnce() -> S + 'static,
    {
        self.solvers.insert(
            SolverKey {
                size,
                metric,
                label,
            },
            LazyCell::new(Box::new(move || Box::new(solver()))),
        );
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
        let size = puzzle.size();
        let SolverContext { metric, label } = *context;

        let key = SolverKey {
            size,
            metric,
            label,
        };

        match self.solvers.get_mut(&key) {
            Some(s) => s.solve_with_config(puzzle, config),
            None => fallback_solver(*context).solve_with_config(puzzle, config),
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
