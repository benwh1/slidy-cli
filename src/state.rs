use std::{error::Error, str::FromStr};

use clap::Subcommand;
use slidy::{
    puzzle::{
        puzzle::Puzzle,
        scrambler::{RandomState, Scrambler},
        sliding_puzzle::SlidingPuzzle,
    },
    solver::{heuristic::ManhattanDistance, solver::Solver},
};

use crate::{size::Size, util::try_func};

#[derive(Subcommand, Debug)]
pub enum Command {
    Generate {
        #[clap(short, long, default_value_t = 1)]
        number: u64,

        #[clap(short, long, default_value_t = Size(4, 4), value_parser = Size::from_str)]
        size: Size,
    },
    Solve {
        state: Option<Puzzle>,
    },
}

pub fn run(command: Command) -> Result<(), Box<dyn Error>> {
    match command {
        Command::Generate { number, size } => generate(number, size),
        Command::Solve { state } => try_func(solve, state),
    }
}

pub fn generate(number: u64, Size(width, height): Size) -> Result<(), Box<dyn Error>> {
    let mut p = Puzzle::new(width as usize, height as usize)?;

    for _ in 0..number {
        p.reset();
        RandomState.scramble(&mut p);
        println!("{p}");
    }

    Ok(())
}

pub fn solve(state: &mut Puzzle) -> Result<(), Box<dyn Error>> {
    let mut s = Solver::new(state, &ManhattanDistance);
    let a = s.solve()?;
    println!("{a}");

    Ok(())
}
