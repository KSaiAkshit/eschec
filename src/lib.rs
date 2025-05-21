use std::cell::OnceCell;
use std::io::{Write, stderr};

// Bit Boards use 64 bits of true or false, to tell if a given peice is at the location.
// 12 Bit boards represent where the chess peices are at all times
pub mod board;
pub mod comms;
pub mod evaluation;
pub mod moves;
pub mod search;
pub mod utils;

pub use board::components::*;
pub use board::*;
use tracing_subscriber::EnvFilter;
pub use utils::cli;
pub use utils::perft;

use clap::Parser;
use cli::{GameCommand, GameSubcommand};
use evaluation::CompositeEvaluator;
use miette::{Context, IntoDiagnostic};
use search::Search;
use tracing::{Level, error, info, span, trace};

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
    let inp_depth = depth;
    let inp_fen = fen.clone();

    let mut board = Board::from_fen(&fen);
    let evaluator = CompositeEvaluator::balanced();
    let mut search = Search::with_time_control(depth, 5_000);
    // let mut search = Search::new(depth);

    let stdin = std::io::stdin();

    println!("{}", board);
    loop {
        let span = span!(Level::DEBUG, "game_loop");
        let _guard = span.enter();

        trace!("inside game_loop");

        print!("{} >> ", board.stm);
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
                    board = Board::from_fen(&inp_fen);
                }
                GameSubcommand::Quit => {
                    info!("Exiting game loop...");
                    break;
                }
                GameSubcommand::Undo => {
                    todo!("Undo last state")
                }
                GameSubcommand::Save { filename } => {
                    todo!("Saving to file: {filename}");
                }
                GameSubcommand::Hint => {
                    info!("Here's a Hint. Support for multiple hints coming soon");
                    let result = search.find_best_move(&board, &evaluator);
                    if let Some((from, to)) = result.best_move {
                        info!("Best move: {} to {} ", from, to);
                        info!(
                            "score: {}, time_taken: {} ms",
                            result.score,
                            result.time_taken.as_millis()
                        );
                    } else {
                        error!("No legal moves available");
                    }
                }
                GameSubcommand::Depth { depth } => {
                    info!("Changing search depth from {} to {}", inp_depth, depth);
                    search.change_depth(depth);
                }
                GameSubcommand::Evaluate => {
                    info!("Evaluating the current board state");
                    let score = board.evaluate_position(&evaluator);
                    info!("Score: {score}");
                }
                GameSubcommand::Clear => {
                    info!("Clearing screen");
                    clear_screen()?;
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

/// Initialize tracing and backtrace
pub fn init() {
    let init: OnceCell<bool> = OnceCell::new();
    init.get_or_init(|| {
        color_backtrace::install();
        tracing_subscriber::fmt()
            .without_time()
            .with_writer(stderr)
            .with_env_filter(EnvFilter::from_default_env().add_directive(Level::TRACE.into()))
            .init();
        true
    });
    if !init.get().unwrap() {
        panic!("Backtrace and/or tracing_subscriber not initialized");
    }
}
