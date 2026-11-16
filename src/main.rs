mod algorithm_ext;
mod args;
mod cache_versions;
mod command;
mod enums;
mod run;
mod solver;
mod util;

use std::process::ExitCode;

use clap::Parser as _;

use crate::{args::Args, run::Runner};

fn main() -> ExitCode {
    #[cfg(unix)]
    #[allow(clippy::undocumented_unsafe_blocks)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let mut runner = Runner::new();
    match runner.run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
