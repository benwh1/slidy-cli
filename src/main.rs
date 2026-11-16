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
    // Rust ignores SIGPIPE by default, which turns writing to a closed pipe (e.g. `slidy generate
    // -n 1000000 | head`) into an error. Restore the default so the process exits quietly instead.
    #[cfg(unix)]
    // SAFETY: This runs before any other threads are started, and SIG_DFL is a valid handler.
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
