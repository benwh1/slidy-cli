use std::str::FromStr;

use clap::Subcommand;
use slidy::algorithm::algorithm::Algorithm;

#[derive(Subcommand, Debug)]
pub enum Command {
    Simplify {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        verbose: bool,
    },
}

pub fn run(command: Command) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Command::Simplify { alg, verbose } => try_simplify(alg, verbose),
    }
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

fn try_simplify(alg: Option<Algorithm>, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(mut alg) = alg {
        simplify(&mut alg, verbose);
    } else {
        for line in std::io::stdin().lines() {
            let mut a = Algorithm::from_str(&line?)?;
            simplify(&mut a, verbose);
        }
    }

    Ok(())
}
