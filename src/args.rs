use clap::Parser;

use crate::command::Command;

const INPUT_FORMATS: &str = "\
Input formats:
  Puzzle states  Rows separated by `/`, pieces separated by spaces, with 0 for the
                 gap, e.g. `1 2 3/4 5 6/7 8 0`.
  Algorithms     Moves U, D, L, R, each the direction a tile moves, optionally
                 followed by a count, e.g. `R3 D2 L` or `RRRDDL`.
  Sizes          `WIDTHxHEIGHT`, e.g. `4x3`, or one number for a square, e.g. `4`.

Commands that take a state or algorithm read them from stdin, one per line, when it
is omitted.";

#[derive(Parser, Debug)]
#[command(
    author, version, about, long_about = None,
    propagate_version = true,
    subcommand_required = true,
    arg_required_else_help = true,
    after_help = INPUT_FORMATS,
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory as _;

    use super::*;

    #[test]
    fn cli_is_valid() {
        Args::command().debug_assert();
    }
}
