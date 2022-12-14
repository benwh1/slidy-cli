use clap::Subcommand;
use slidy::algorithm::algorithm::Algorithm;

use crate::util::try_func;

#[derive(Subcommand, Debug)]
pub enum Command {
    Length {
        alg: Option<Algorithm>,
    },
    Simplify {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        verbose: bool,
    },
    Invert {
        alg: Option<Algorithm>,
    },
    Prepend {
        state: Option<Algorithm>,

        #[clap(short, long)]
        prefix: Algorithm,
    },
    Append {
        state: Option<Algorithm>,

        #[clap(short, long)]
        suffix: Algorithm,
    },
}

pub fn run(command: Command) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Command::Length { alg } => try_func(length, alg),
        Command::Simplify { alg, verbose } => try_func(|a| simplify(a, verbose), alg),
        Command::Invert { alg } => try_func(invert, alg),
        Command::Prepend { state, prefix } => try_func(|a| prepend(a, &prefix), state),
        Command::Append { state, suffix } => try_func(|a| append(a, &suffix), state),
    }
}

fn length(alg: &mut Algorithm) {
    println!("{}", alg.len());
}

fn simplify(alg: &mut Algorithm, verbose: bool) {
    let orig = alg.len();
    alg.simplify();
    let new = alg.len();

    println!("{alg}");
    if verbose {
        println!("Original length: {orig}");

        let diff = orig - new;
        let percent = diff as f32 * 100.0 / orig as f32;
        println!("New length: {new} [-{diff}, -{percent:.4}%]",);
    }
}

fn invert(alg: &mut Algorithm) {
    alg.invert();
    println!("{alg}");
}

fn prepend(alg: &mut Algorithm, prefix: &Algorithm) {
    print!("{prefix}{alg}\n");
}

fn append(alg: &mut Algorithm, suffix: &Algorithm) {
    print!("{alg}{suffix}\n");
}
