use std::{error::Error, str::FromStr};

use clap::{ArgGroup, Subcommand};
use slidy::{
    algorithm::algorithm::Algorithm,
    puzzle::{
        puzzle::Puzzle,
        scrambler::{RandomMoves, RandomState, Scrambler},
        sliding_puzzle::SlidingPuzzle,
    },
    solver::{heuristic::ManhattanDistance, solver::Solver},
};

use crate::{size::Size, util::try_func};

#[derive(Subcommand, Debug)]
pub enum Command {
    #[clap(group(ArgGroup::new("scrambler")))]
    #[clap(group(ArgGroup::new("scrambler_random_moves").requires("random_moves").multiple(true)))]
    Generate {
        #[clap(short, long, default_value_t = 1)]
        number: u64,

        #[clap(short, long, default_value_t = Size(4, 4), value_parser = Size::from_str)]
        size: Size,

        #[clap(long, group = "scrambler", default_value_t = true)]
        random_state: bool,

        #[clap(long, group = "scrambler")]
        random_moves: bool,

        #[clap(short = 'm', long, default_value_t = 80, requires = "random_moves")]
        num_moves: u64,

        #[clap(short = 'b', long, requires = "random_moves")]
        allow_backtracking: bool,

        #[clap(short = 'i', long, requires = "random_moves")]
        allow_illegal_moves: bool,
    },
    Solve {
        state: Option<Puzzle>,
    },
    Solvable {
        state: Option<Puzzle>,
    },
    Apply {
        state: Option<Puzzle>,

        #[clap(short, long)]
        alg: Algorithm,
    },
}

pub fn run(command: Command) -> Result<(), Box<dyn Error>> {
    match command {
        Command::Generate {
            number,
            size,
            random_moves,
            num_moves,
            allow_backtracking,
            allow_illegal_moves,
            ..
        } => {
            if random_moves {
                generate(
                    number,
                    size,
                    RandomMoves {
                        moves: num_moves,
                        allow_backtracking,
                        allow_illegal_moves,
                    },
                )
            } else {
                generate(number, size, RandomState)
            }
        }
        Command::Solve { state } => try_func(solve, state),
        Command::Solvable { state } => try_func(solvable, state),
        Command::Apply { state, alg } => try_func(|s| apply(s, &alg), state),
    }
}

pub fn generate(
    number: u64,
    Size(width, height): Size,
    s: impl Scrambler,
) -> Result<(), Box<dyn Error>> {
    let mut p = Puzzle::new(width as usize, height as usize)?;

    for _ in 0..number {
        p.reset();
        s.scramble(&mut p);
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

pub fn solvable(state: &mut Puzzle) {
    println!("{}", state.is_solvable());
}

pub fn apply(state: &mut Puzzle, alg: &Algorithm) {
    if state.try_apply_alg(alg) {
        println!("{state}");
    } else {
        println!("Invalid");
    }
}
