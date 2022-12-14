use std::str::FromStr;

use clap::Subcommand;
use slidy::algorithm::algorithm::Algorithm;

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
}

pub fn run(command: Command) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Command::Length { alg } => try_func(length, alg),
        Command::Simplify { alg, verbose } => try_func(|a| simplify(a, verbose), alg),
        Command::Invert { alg } => try_func(invert, alg),
    }
}

fn try_func<F: Fn(&mut Algorithm)>(
    f: F,
    alg: Option<Algorithm>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(mut alg) = alg {
        f(&mut alg);
    } else {
        for line in std::io::stdin().lines() {
            let mut alg = Algorithm::from_str(&line?)?;
            f(&mut alg);
        }
    }

    Ok(())
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
