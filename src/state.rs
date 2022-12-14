use clap::Subcommand;
use slidy::puzzle::{
    puzzle::Puzzle,
    scrambler::{RandomState, Scrambler},
    sliding_puzzle::SlidingPuzzle,
};

#[derive(Subcommand, Debug)]
pub enum Command {
    Generate {
        #[clap(short, long, default_value_t = 1)]
        number: u64,

        #[clap(short, long, default_value_t = 4)]
        size: u32,
    },
}

pub fn run(command: Command) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Command::Generate { number, size } => generate(number, size),
    }
}

pub fn generate(number: u64, size: u32) -> Result<(), Box<dyn std::error::Error>> {
    let mut p = Puzzle::new(size as usize, size as usize)?;

    for _ in 0..number {
        p.reset();
        RandomState.scramble(&mut p);
        println!("{p}");
    }

    Ok(())
}
