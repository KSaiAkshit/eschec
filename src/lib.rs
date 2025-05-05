use std::cell::OnceCell;
use std::io::Write;

use miette::{Context, IntoDiagnostic};

// Bit Boards use 64 bits of true or false, to tell if a given peice is at the location.
// 12 Bit boards represent where the chess peices are at all times
pub mod board;
pub mod evaluation;
pub mod moves;
pub mod perft;
pub mod search;

pub use board::components::*;
pub use board::*;
use tracing::Level;

// pub struct Game {
//     board: Board,
//     evaluator: CompositeEvaluator,
//     search: Search,
// }

// impl Game {
//     pub fn new() -> Self {
//         Self {
//             board: Board::new(),
//             evaluator: CompositeEvaluator::balanced(),
//             search: Search::new(3),
//         }
//     }
// }

// impl Default for Game {
//     fn default() -> Self {
//         Self::new()
//     }
// }

pub fn clear_screen() -> miette::Result<()> {
    print!("\x1b[2J\x1b[1H");
    std::io::stdout()
        .flush()
        .into_diagnostic()
        .context("Flushing stdout")
}

pub fn get_input(input: &str) -> miette::Result<(Square, Square)> {
    // Remove any trailing newline or spaces
    let trimmed = input.trim();

    miette::ensure!(!trimmed.is_empty(), "Empty input given");

    let mut parts = trimmed.split_whitespace();
    let from = parts.next().context("Missing 'from' square")?;
    let to = parts.next().context("Missing 'to' square")?;

    let from_pos: usize = from
        .parse()
        .into_diagnostic()
        .with_context(|| format!("Invalid 'from' position: {}", from))?;
    let to_pos: usize = to
        .parse()
        .into_diagnostic()
        .with_context(|| format!("Invalid 'to' position: {}", to))?;

    let from_square = Square::new(from_pos)
        .with_context(|| format!("'from' Square out of bounds: {}", from_pos))?;
    let to_square =
        Square::new(to_pos).with_context(|| format!("'to' Square out of bounds: {}", to_pos))?;

    Ok((from_square, to_square))
}

pub fn init() {
    let init: OnceCell<bool> = OnceCell::new();
    init.get_or_init(|| {
        color_backtrace::install();
        tracing_subscriber::fmt()
            .with_max_level(Level::DEBUG)
            .init();
        true
    });
    if !init.get().unwrap() {
        panic!("Backtrace and/or tracing_subscriber not initialized");
    }
}
