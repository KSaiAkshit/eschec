#![feature(portable_simd)]
use std::env;
use std::fs::File;
use std::io::{Write, stderr};
use std::path::Path;
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
use tracing_appender::non_blocking;
use tracing_subscriber::{
    EnvFilter, Layer, fmt, layer::SubscriberExt, reload::Handle, util::SubscriberInitExt,
};
pub use utils::cli;
pub use utils::perft;

use chrono::Local;
use clap::Parser;
use cli::{GameCommand, GameSubcommand};
use evaluation::CompositeEvaluator;
use miette::{Context, IntoDiagnostic};
use search::Search;
use tracing::{Level, error, info, span, trace};

use crate::moves::move_info::Move;

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
pub const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

pub mod consts {
    use crate::{Piece, Side};

    pub const NUM_SIDES: usize = Side::SIDES.len();
    pub const NUM_PIECES: usize = Piece::PIECES.len();
    pub const NUM_SQUARES: usize = 64;
    pub const NUM_CASTLING_RIGHTS: usize = 16;
    pub const NUM_FILES: usize = 8;
}

static LOG_FILTER_HANDLE: LazyLock<Mutex<Handle<EnvFilter, tracing_subscriber::Registry>>> =
    LazyLock::new(|| {
        color_backtrace::install();
        let filter = match env::var("RUST_LOG") {
            Ok(env_filter) => EnvFilter::new(env_filter),
            Err(_) => EnvFilter::new("info"),
        };

        let (filter, handle) = tracing_subscriber::reload::Layer::new(filter);

        let log_dir = Path::new("/tmp/eschec_logs");
        if !log_dir.exists() {
            std::fs::create_dir(log_dir).expect("Failed to create log directory");
        }

        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
        let log_filename = format!("/tmp/eschec_logs/eschec_{timestamp}.log");
        let log_file = File::create(&log_filename)
            .unwrap_or_else(|_| panic!("Failed to create log file: {log_filename}"));

        let (non_blocking_writer, _guard) = non_blocking(log_file);
        std::mem::forget(_guard); // Keep the guard alive.

        let file_layer = fmt::layer()
            .with_writer(non_blocking_writer)
            .with_ansi(true)
            .with_filter(EnvFilter::new("trace"));

        let env_filter = EnvFilter::builder()
            .with_default_directive(Level::INFO.into())
            .from_env_lossy();
        let console_layer = fmt::layer()
            .without_time()
            .with_writer(stderr)
            .with_filter(env_filter);

        tracing_subscriber::registry()
            .with(filter) // This filter is controlled by the handle
            .with(console_layer)
            .with(file_layer)
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

pub fn game_loop(fen: String, depth: u8) -> miette::Result<()> {
    let inp_depth = depth;
    let inp_fen = fen.clone();

    let mut board = Board::from_fen(&fen);
    let evaluator = CompositeEvaluator::balanced();
    let mut search = Search::with_time_control(depth, 10_000);
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
                    let mov = match Move::from_uci(&board, &move_str) {
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
                    let result = search.find_best_move(&board, &evaluator, None);
                    if let Some(mov) = result.best_move {
                        info!("Best move: {} ", mov.uci());
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
                        search.change_depth(depth)?;
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
