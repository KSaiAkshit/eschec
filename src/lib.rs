use std::io::Write;

use anyhow::Context;
use board::components::Square;

// Bit Boards use 64 bits of true or false, to tell if a given peice is at the location.
// 12 Bit boards represent where the chess peices are at all times
pub mod board;

pub fn clear_screen() -> anyhow::Result<()> {
    print!("\x1b[2J\x1b[1H");
    std::io::stdout().flush().context("Flushing stdout")
}

pub fn get_input(input: &str) -> anyhow::Result<(Square, Square)> {
    // Remove any trailing newline or spaces
    let trimmed = input.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Empty input given");
    }

    let (from, to) = match trimmed.split_once(' ') {
        Some((f, t)) => (f, t),
        None => {
            anyhow::bail!("Invalid input format. Expected 'from to', got: {input}");
        }
    };

    let from_pos: usize = match from.parse() {
        Ok(num) => num,
        Err(_) => {
            anyhow::bail!("Invalid 'from' position: {}", from);
        }
    };

    let to_pos: usize = match to.parse() {
        Ok(num) => num,
        Err(_) => {
            anyhow::bail!("Invalid 'to' position: {}", to);
        }
    };

    let from_square = match Square::new(from_pos) {
        Some(square) => square,
        None => {
            anyhow::bail!("Invalid 'from' square: {}", from_pos);
        }
    };

    let to_square = match Square::new(to_pos) {
        Some(square) => square,
        None => {
            anyhow::bail!("Invalid 'to' square: {}", to_pos);
        }
    };

    Ok((from_square, to_square))
}
