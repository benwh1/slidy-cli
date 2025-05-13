use std::cell::OnceCell;

use slidy::{
    algorithm::algorithm::Algorithm,
    puzzle::{
        label::label::RowGrids, puzzle::Puzzle, size::Size, sliding_puzzle::SlidingPuzzle as _,
    },
    solver::{
        heuristic::manhattan::ManhattanDistance, pdb4443::solver::Solver as Pdb4443Solver,
        solver::Solver,
    },
};

pub struct State {
    solver4x4: OnceCell<Pdb4443Solver>,
}

impl State {
    pub fn new() -> Self {
        Self {
            solver4x4: OnceCell::new(),
        }
    }

    pub fn solve(&self, puzzle: &Puzzle) -> Algorithm {
        if puzzle.size() == Size::new(4, 4).unwrap() {
            self.solver4x4
                .get_or_init(Pdb4443Solver::new)
                .solve(puzzle)
                .unwrap()
        } else {
            Solver::new(&ManhattanDistance(&RowGrids), &RowGrids)
                .solve(puzzle)
                .unwrap()
        }
    }
}
