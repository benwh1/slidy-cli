#![feature(int_roundings)]

mod args;
mod command;
mod enums;
mod run;
mod util;

use std::error::Error;

use clap::Parser as _;

use crate::args::Args;

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    run::run(Args::parse())
}
