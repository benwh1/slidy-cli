use crate::solver::Solver;

pub struct State {
    pub solver: Solver,
}

impl State {
    pub fn new() -> Self {
        Self {
            solver: Solver::new(),
        }
    }
}
