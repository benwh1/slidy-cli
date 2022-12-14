pub mod size;
pub mod state;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    author, version, about, long_about = None,
    propagate_version = true,
    subcommand_required = true,
    arg_required_else_help = true,
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(subcommand)]
    State(state::Command),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Command::State(c) => state::run(c),
    }
}
