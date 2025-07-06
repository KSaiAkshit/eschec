#![feature(portable_simd)]
use std::env;
use std::io::{Write, stderr};
use std::str::FromStr;
use std::sync::{LazyLock, Mutex};

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
use tracing_subscriber::{
    EnvFilter, layer::SubscriberExt, reload::Handle, util::SubscriberInitExt,
};
pub use utils::cli;
pub use utils::perft;

use clap::Parser;
use cli::{GameCommand, GameSubcommand};
use evaluation::CompositeEvaluator;
use miette::{Context, IntoDiagnostic};
use search::Search;
use tracing::{Level, error, info, span, trace};

use crate::moves::move_info::Move;

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
pub const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

static LOG_FILTER_HANDLE: LazyLock<Mutex<Handle<EnvFilter, tracing_subscriber::Registry>>> =
    LazyLock::new(|| {
        color_backtrace::install();
        let filter = match env::var("RUST_LOG") {
            Ok(env_filter) => EnvFilter::new(env_filter),
            Err(_) => EnvFilter::new("info"),
        };

        let (filter, handle) = tracing_subscriber::reload::Layer::new(filter);
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .without_time()
                    .with_writer(stderr),
            )
            .init();
        Mutex::new(handle)
    });

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
        .with_context(|| format!("Invalid 'from' position: {from}"))?;
    let to_pos: usize = to
        .parse()
        .into_diagnostic()
        .with_context(|| format!("Invalid 'to' position: {to}"))?;

    let from_square = Square::new(from_pos)
        .with_context(|| format!("'from' Square out of bounds: {from_pos}"))?;
    let to_square =
        Square::new(to_pos).with_context(|| format!("'to' Square out of bounds: {to_pos}"))?;

    Ok((from_square, to_square))
}


fn parse_uci_move(board: &Board, uci: &str) -> miette::Result<Move> {
    if uci.len() < 4 || uci.len() > 5 {
        miette::bail!("Invalid UCI move format: '{}'", uci);
    }
    let from_str = &uci[0..2];
    let to_str = &uci[2..4];
    let promo_char = uci.chars().nth(4);

    let from = Square::from_str(from_str)?;
    let to = Square::from_str(to_str)?;

    // Find the matching legal move. This is the only way to get the correct flags.
    let legal_moves = board.generate_legal_moves();
    let found_move = legal_moves.into_iter().find(|m| {
        if m.from_sq() == from && m.to_sq() == to {
            // If there's a promotion, make sure it matches.
            if let Some(pc) = promo_char {
                return m.promoted_piece_char() == Some(pc);
            }
            // If no promotion in UCI string, match a non-promotion move.
            return !m.is_promotion();
        }
        false
    });

    found_move.context(format!(
        "The move '{uci}' is not legal in the current position."
    ))
}

pub fn game_loop(fen: String, depth: u8) -> miette::Result<()> {
    let inp_depth = depth;
    let inp_fen = fen.clone();

    let mut board = Board::from_fen(&fen);
    let evaluator = CompositeEvaluator::balanced();
    let mut search = Search::with_time_control(depth, 5_000);
    // let mut search = Search::new(depth);

    let stdin = std::io::stdin();

    println!("{board}");
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
                eprintln!("Error parsing input: {e}");
                continue;
            }
        };

        match GameCommand::try_parse_from(args) {
            Ok(game_cmd) => match game_cmd.cmd {
                GameSubcommand::Move { move_str } => {
                    let mov = match parse_uci_move(&board, &move_str) {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("{e:?}");
                            continue;
                        }
                    };
                    info!("Attempting move: {}", mov);
                    // Already verified that mov is legal when parsing for uci move above
                    if let Err(e) = board.make_move(mov) {
                        eprintln!("{e:?}");
                        continue;
                    }
                }
                GameSubcommand::Print => {
                    info!("Printing board..");
                    println!("{board}");
                }
                GameSubcommand::Perft { depth, divide } => {
                    info!(
                        "Running perft to depth {}, with divide: {}",
                        depth.unwrap_or(5),
                        divide
                    );
                    let mut board_copy = board;
                    if divide {
                        perft::perft_divide(&mut board_copy, depth.unwrap_or(5));
                    } else {
                        perft::run_perft_suite(&mut board_copy, depth.unwrap_or(5));
                    }
                }
                GameSubcommand::Restart => {
                    info!("Restarting game...");
                    board = Board::from_fen(&inp_fen);
                }
                GameSubcommand::Fen => {
                    info!("Printing fen...");
                    println!("{}", fen::to_fen(&board)?);
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
                    if let Some(mov) = result.best_move {
                        info!("Best move: {} ", mov);
                        info!(
                            "score: {}, time_taken: {} ms",
                            result.score,
                            result.time_taken.as_millis()
                        );
                    } else {
                        error!("No legal moves available");
                    }
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
                GameSubcommand::Set { cmd } => match cmd {
                    cli::SetSubcommand::Fen { parts } => {
                        if parts.is_empty() {
                            error!("No FEN string provided. Usage: set fen <FEN_STRING>");
                            continue;
                        }
                        let fen_str = parts.join(" ");
                        info!("Setting fen to {fen_str}");
                        board = Board::from_fen(&fen_str);
                        println!("{board}");
                    }
                    cli::SetSubcommand::Depth { depth } => {
                        info!("Changing search depth from {inp_depth} to {depth}");
                        search.change_depth(depth);
                    }
                    cli::SetSubcommand::LogLevel { level } => {
                        let new_level: Level = level.into();
                        info!("Setting log level to {new_level}");
                        if let Err(e) = set_log_level(new_level) {
                            error!("Failed to set log level: {e:?}");
                        }
                    }
                },
            },
            Err(e) => {
                // println!("{}", e.render());
                e.print().expect("Failed to print clap error");
            }
        }
    }

    Ok(())
}

pub fn set_log_level(level: Level) -> miette::Result<()> {
    let new_filter = EnvFilter::new(level.to_string());

    LOG_FILTER_HANDLE
        .lock()
        .unwrap()
        .modify(|filter| *filter = new_filter)
        .into_diagnostic()
        .with_context(|| format!("Failed to modify log filter to level: {level}"))
}

/// Initialize tracing and backtrace
pub fn init() {
    LazyLock::force(&LOG_FILTER_HANDLE);
    #[cfg(feature = "simd")]
    {
        info!("Using Simd");
    }
    #[cfg(not(feature = "simd"))]
    {
        info!("Not using Simd");
    }
}
