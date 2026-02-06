mod algorithm_ext;
mod args;
mod command;
mod enums;
mod run;
mod state;
mod util;

use std::error::Error;

use clap::Parser as _;

use crate::{args::Args, run::Runner};

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let runner = Runner::new();
    runner.run(Args::parse())
}
