use clap::Parser;

use crate::command::Command;

#[derive(Parser, Debug)]
#[command(
    author, version, about, long_about = None,
    propagate_version = true,
    subcommand_required = true,
    arg_required_else_help = true,
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}
