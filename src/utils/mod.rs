pub mod cli;
pub mod log;
pub mod perft;
pub mod prng;
pub mod sts_runner;

use std::io::Write;

use miette::{Context, IntoDiagnostic};

pub fn clear_screen() -> miette::Result<()> {
    print!("\x1b[2J\x1b[1H");
    std::io::stdout()
        .flush()
        .into_diagnostic()
        .context("Flushing stdout")
}
