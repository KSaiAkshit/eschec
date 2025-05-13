#![allow(unused)]
use std::cell::OnceCell;
use std::io::Write;

use clap::Parser;
use cli::{GameCommand, GameSubcommand};
use evaluation::CompositeEvaluator;
use miette::{Context, IntoDiagnostic};
use search::Search;
use tracing::{debug, info, span};

// Bit Boards use 64 bits of true or false, to tell if a given peice is at the location.
// 12 Bit boards represent where the chess peices are at all times
pub mod board;
pub mod cli;
pub mod comms;
pub mod evaluation;
pub mod moves;
pub mod perft;
pub mod search;

pub use board::components::*;
pub use board::*;
use tracing::Level;

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

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

fn parse_move_input(from: String, to: String) -> miette::Result<(Square, Square)> {
    let from_square: Square = from.try_into()?;
    let to_square: Square = to.try_into()?;

    Ok((from_square, to_square))
}

pub fn game_loop(fen: String, depth: u8) -> miette::Result<()> {
    info!("Starting game loop with FEN: {} and depth: {}", fen, depth);

    let mut board = Board::from_fen(&fen);
    let evaluator = CompositeEvaluator::balanced();
    let mut search = Search::new(depth);

    let stdin = std::io::stdin();

    loop {
        let span = span!(Level::DEBUG, "game_loop");
        let _guard = span.enter();

        debug!("inside game_loop");

        println!("{}", board);

        print!("{} > ", board.stm);
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        if stdin.read_line(&mut input).unwrap() == 0 {
            println!("EOF detected, exiting...");
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let args = match shell_words::split(input) {
            Ok(tokens) => tokens,
            Err(e) => {
                eprintln!("Error parsing input: {}", e);
                continue;
            }
        };

        match GameCommand::try_parse_from(args) {
            Ok(game_cmd) => match game_cmd.cmd {
                GameSubcommand::Move { from, to } => {
                    info!("Moving from {} to {}", from, to);

                    let (from_square, to_square) = match parse_move_input(from, to) {
                        Ok(f) => (f.0, f.1),
                        Err(e) => {
                            eprintln!("{:?}", e);
                            continue;
                        }
                    };
                    if let Err(e) = board.try_move(from_square, to_square) {
                        eprintln!("{:?}", e);
                        continue;
                    }
                }
                GameSubcommand::Print => {
                    info!("Printing board..");
                    println!("{}", board);
                }
                GameSubcommand::Perft { depth } => {
                    info!("Running perft to depth {}", depth.unwrap_or(5));
                    let mut board_copy = board;
                    perft::run_perft_suite(&mut board_copy, depth.unwrap_or(5));
                }
                GameSubcommand::Restart => {
                    info!("Restarting game...");
                }
                GameSubcommand::Quit => {
                    info!("Exiting game loop...");
                    break;
                }
            },
            Err(e) => {
                // println!("{}", e.render());
                e.print().expect("Failed to print clap error");
            }
        }
    }

    Ok(())
}

pub fn init() {
    let init: OnceCell<bool> = OnceCell::new();
    init.get_or_init(|| {
        color_backtrace::install();
        tracing_subscriber::fmt()
            .without_time()
            .with_max_level(Level::TRACE)
            .init();
        true
    });
    if !init.get().unwrap() {
        panic!("Backtrace and/or tracing_subscriber not initialized");
    }
}
