use std::cell::OnceCell;

use directories::ProjectDirs;
use slidy::{
    algorithm::algorithm::Algorithm,
    puzzle::{label::label::RowGrids, puzzle::Puzzle, sliding_puzzle::SlidingPuzzle as _},
    solver::{
        heuristic::manhattan::ManhattanDistance,
        small::pdb::{
            Pdb2x2Mtm, Pdb2x2Stm, Pdb3x2Mtm, Pdb3x2Stm, Pdb3x3Mtm, Pdb3x3Stm, Pdb4x2Mtm, Pdb4x2Stm,
            Pdb4x3Mtm, Pdb4x3Stm, Pdb5x2Mtm, Pdb5x2Stm, Pdb6x2Mtm, Pdb6x2Stm,
        },
        solver::Solver,
        Solver2x2Mtm, Solver2x2Stm, Solver3x2Mtm, Solver3x2Stm, Solver3x3Mtm, Solver3x3Stm,
        Solver4x2Mtm, Solver4x2Stm, Solver4x3Mtm, Solver4x3Stm, Solver4x4Mtm, Solver4x4Stm,
        Solver5x2Mtm, Solver5x2Stm, Solver6x2Mtm, Solver6x2Stm,
    },
};

use crate::enums::Metric;

pub struct State {
    solver_2x2_stm: OnceCell<Solver2x2Stm>,
    solver_3x2_stm: OnceCell<Solver3x2Stm>,
    solver_3x3_stm: OnceCell<Solver3x3Stm>,
    solver_4x2_stm: OnceCell<Solver4x2Stm>,
    solver_4x3_stm: OnceCell<Solver4x3Stm>,
    solver_4x4_stm: OnceCell<Solver4x4Stm>,
    solver_5x2_stm: OnceCell<Solver5x2Stm>,
    solver_6x2_stm: OnceCell<Solver6x2Stm>,

    solver_2x2_mtm: OnceCell<Solver2x2Mtm>,
    solver_3x2_mtm: OnceCell<Solver3x2Mtm>,
    solver_3x3_mtm: OnceCell<Solver3x3Mtm>,
    solver_4x2_mtm: OnceCell<Solver4x2Mtm>,
    solver_4x3_mtm: OnceCell<Solver4x3Mtm>,
    solver_4x4_mtm: OnceCell<Solver4x4Mtm>,
    solver_5x2_mtm: OnceCell<Solver5x2Mtm>,
    solver_6x2_mtm: OnceCell<Solver6x2Mtm>,
}

impl State {
    pub fn new() -> Self {
        Self {
            solver_2x2_stm: OnceCell::new(),
            solver_3x2_stm: OnceCell::new(),
            solver_3x3_stm: OnceCell::new(),
            solver_4x2_stm: OnceCell::new(),
            solver_4x3_stm: OnceCell::new(),
            solver_4x4_stm: OnceCell::new(),
            solver_5x2_stm: OnceCell::new(),
            solver_6x2_stm: OnceCell::new(),

            solver_2x2_mtm: OnceCell::new(),
            solver_3x2_mtm: OnceCell::new(),
            solver_3x3_mtm: OnceCell::new(),
            solver_4x2_mtm: OnceCell::new(),
            solver_4x3_mtm: OnceCell::new(),
            solver_4x4_mtm: OnceCell::new(),
            solver_5x2_mtm: OnceCell::new(),
            solver_6x2_mtm: OnceCell::new(),
        }
    }

    pub fn solve(&self, puzzle: &Puzzle, metric: Metric) -> Algorithm {
        let mut pdb_cache_dir = ProjectDirs::from("", "", "slidy-cli")
            .unwrap()
            .cache_dir()
            .to_path_buf();
        pdb_cache_dir.push("solver");
        pdb_cache_dir.push("pdb");

        std::fs::create_dir_all(&pdb_cache_dir).unwrap();

        let (w, h) = puzzle.size().into();

        macro_rules! solve {
            ($field:ident, $pdb_file:literal, $pdb_ty:ty, $solver_ty:ty) => {{
                self.$field
                    .get_or_init(|| {
                        type PdbTy = $pdb_ty;
                        type SolverTy = $solver_ty;

                        let pdb_file = pdb_cache_dir.join($pdb_file);

                        let pdb = std::fs::read(&pdb_file).map_or_else(
                            |_| {
                                let pdb = PdbTy::new();
                                std::fs::write(&pdb_file, pdb.as_ref()).unwrap();
                                pdb
                            },
                            |bytes| {
                                // SAFETY: this computes a checksum to verify correctness, which is
                                // good enough here.
                                unsafe { PdbTy::try_from_bytes(bytes.into_boxed_slice()) }
                                    .unwrap_or_else(|| {
                                        let pdb = PdbTy::new();
                                        std::fs::write(&pdb_file, pdb.as_ref()).unwrap();
                                        pdb
                                    })
                            },
                        );

                        SolverTy::with_pdb(pdb)
                    })
                    .solve(puzzle)
                    .unwrap()
            }};
        }
        match metric {
            Metric::Stm => match (w, h) {
                (2, 2) => solve!(solver_2x2_stm, "2x2-stm.bin", Pdb2x2Stm, Solver2x2Stm),
                (3, 2) | (2, 3) => solve!(solver_3x2_stm, "3x2-stm.bin", Pdb3x2Stm, Solver3x2Stm),
                (3, 3) => solve!(solver_3x3_stm, "3x3-stm.bin", Pdb3x3Stm, Solver3x3Stm),
                (4, 2) | (2, 4) => solve!(solver_4x2_stm, "4x2-stm.bin", Pdb4x2Stm, Solver4x2Stm),
                (4, 3) | (3, 4) => solve!(solver_4x3_stm, "4x3-stm.bin", Pdb4x3Stm, Solver4x3Stm),
                (5, 2) | (2, 5) => solve!(solver_5x2_stm, "5x2-stm.bin", Pdb5x2Stm, Solver5x2Stm),
                (6, 2) | (2, 6) => solve!(solver_6x2_stm, "6x2-stm.bin", Pdb6x2Stm, Solver6x2Stm),
                (4, 4) => self
                    .solver_4x4_stm
                    .get_or_init(Solver4x4Stm::new)
                    .solve(puzzle)
                    .unwrap(),
                _ => Solver::new(&ManhattanDistance(&RowGrids), &RowGrids)
                    .solve(puzzle)
                    .unwrap(),
            },
            Metric::Mtm => match (w, h) {
                (2, 2) => solve!(solver_2x2_mtm, "2x2-mtm.bin", Pdb2x2Mtm, Solver2x2Mtm),
                (3, 2) | (2, 3) => solve!(solver_3x2_mtm, "3x2-mtm.bin", Pdb3x2Mtm, Solver3x2Mtm),
                (3, 3) => solve!(solver_3x3_mtm, "3x3-mtm.bin", Pdb3x3Mtm, Solver3x3Mtm),
                (4, 2) | (2, 4) => solve!(solver_4x2_mtm, "4x2-mtm.bin", Pdb4x2Mtm, Solver4x2Mtm),
                (4, 3) | (3, 4) => solve!(solver_4x3_mtm, "4x3-mtm.bin", Pdb4x3Mtm, Solver4x3Mtm),
                (5, 2) | (2, 5) => solve!(solver_5x2_mtm, "5x2-mtm.bin", Pdb5x2Mtm, Solver5x2Mtm),
                (6, 2) | (2, 6) => solve!(solver_6x2_mtm, "6x2-mtm.bin", Pdb6x2Mtm, Solver6x2Mtm),
                (4, 4) => self
                    .solver_4x4_mtm
                    .get_or_init(|| {
                        let pdb_file = pdb_cache_dir.join("4x4-mtm.bin");

                        let make_solver = || {
                            let solver = Solver4x4Mtm::new();
                            std::fs::write(&pdb_file, solver.pdb_bytes()).unwrap();
                            solver
                        };

                        std::fs::read(&pdb_file).map_or_else(
                            |_| make_solver(),
                            |bytes| {
                                // SAFETY: this computes a checksum to verify correctness, which is
                                // good enough here.
                                unsafe {
                                    Solver4x4Mtm::try_with_pdb_bytes(bytes.into_boxed_slice())
                                }
                                .unwrap_or_else(make_solver)
                            },
                        )
                    })
                    .solve(puzzle)
                    .unwrap(),
                _ => todo!("solving {w}x{h} in MTM is not yet supported"),
            },
        }
    }
}
