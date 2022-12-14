use std::str::FromStr;

use clap::Subcommand;
use slidy::puzzle::{
    puzzle::Puzzle,
    scrambler::{RandomState, Scrambler},
    sliding_puzzle::SlidingPuzzle,
};

use crate::size::Size;

#[derive(Subcommand, Debug)]
pub enum Command {
    Generate {
        #[clap(short, long, default_value_t = 1)]
        number: u64,

        #[clap(short, long, default_value_t = Size(4, 4), value_parser = Size::from_str)]
        size: Size,
    },
}

pub fn run(command: Command) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Command::Generate { number, size } => generate(number, size),
    }
}

pub fn generate(number: u64, Size(width, height): Size) -> Result<(), Box<dyn std::error::Error>> {
    let mut p = Puzzle::new(width as usize, height as usize)?;

    for _ in 0..number {
        p.reset();
        RandomState.scramble(&mut p);
        println!("{p}");
    }

    Ok(())
}
