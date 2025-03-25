use std::io::Write;

use anyhow::Context;

// Bit Boards use 64 bits of true or false, to tell if a given peice is at the location.
// 12 Bit boards represent where the chess peices are at all times
pub mod board;

pub fn clear_screen() -> anyhow::Result<()> {
    print!("\x1b[2J\x1b[1H");
    std::io::stdout().flush().context("Flushing stdout")
}
