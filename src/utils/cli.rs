use std::io::Write;

use clap::{Parser, Subcommand, ValueEnum};
use tracing::{Level, span};

use crate::prelude::*;

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"), version = env!("CARGO_PKG_VERSION"), about = env!("CARGO_PKG_DESCRIPTION") )]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start game with given FEN and depth, or use default fen
    Play {
        /// FEN string for starting position
        #[arg(short, long, default_value = START_FEN)]
        fen: Option<String>,
        /// set search depth
        #[arg(short, long, default_value = "5")]
        depth: Option<u8>,
    },

    /// Run perft on game with given FEN and depth, or use default fen
    Perft {
        /// FEN string for starting position
        #[arg(short, long, default_value = START_FEN)]
        fen: Option<String>,
        /// set search depth
        #[arg(short, long, default_value = "5")]
        depth: u8,
        /// set divide flag
        #[arg(long, default_value = "false")]
        divide: bool,
    },

    /// Run headless to play with GUI, optionally selecting a protocol
    Headless {
        #[arg(short, long, default_value = "uci")]
        protocol: Option<String>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for Level {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum SetSubcommand {
    /// Set the board position using a FEN string
    Fen {
        /// The FEN string. Consumes the rest of the line.
        parts: Vec<String>,
    },
    /// Change the AI search depth
    Depth { depth: u8 },
    /// Change the logging level
    #[clap(visible_alias = "log")]
    LogLevel { level: LogLevel },
    /// Enable or diable logging to a file
    LogFile { enable: String },
}

#[derive(Parser, Debug)]
#[command(name = "game_cmd", no_binary_name = true)]
pub struct GameCommand {
    #[command(subcommand)]
    pub cmd: GameSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum GameSubcommand {
    /// Make a move on the board
    #[clap(visible_alias = "m")]
    Move { move_str: String },

    /// Print the current board state
    #[clap(visible_alias = "p")]
    Print,

    /// Undo the last move
    #[clap(visible_alias = "u")]
    Undo,

    /// Save the current game to a PGN file
    #[clap(visible_alias = "s")]
    Save { filename: String },

    /// Show hints (top 3 best moves with evaluation)
    #[clap(visible_alias = "h")]
    Hint,

    /// Show the current evaluation of the piece
    #[clap(visible_alias = "e")]
    Evaluate,

    /// Show the current fen of the board
    #[clap(visible_alias = "f")]
    Fen,

    /// Run a perft test with given depth [default: 5]
    #[clap(visible_alias = "pe")]
    Perft {
        depth: Option<u8>,
        #[arg(short, default_value = "false")]
        divide: bool,
    },

    /// Change a setting (fen, depth, log-level)
    Set {
        #[command(subcommand)]
        cmd: SetSubcommand,
    },

    /// Clear screen
    #[clap(visible_alias = "c")]
    Clear,

    /// Restart game with same fen
    #[clap(visible_alias = "r")]
    Restart,

    /// Quit game
    #[clap(visible_alias = "q")]
    Quit,
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
                        perft_divide(&mut board_copy, depth.unwrap_or(5));
                    } else {
                        run_perft_suite(&mut board_copy, depth.unwrap_or(5));
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
                            "score: {}, time_taken: {} ms, nodes: {}, pruned: {}",
                            result.score,
                            result.time_taken.as_millis(),
                            result.nodes_searched,
                            result.pruned_nodes
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
                    utils::clear_screen()?;
                }
                GameSubcommand::Set { cmd } => match cmd {
                    SetSubcommand::Fen { parts } => {
                        if parts.is_empty() {
                            error!("No FEN string provided. Usage: set fen <FEN_STRING>");
                            continue;
                        }
                        let fen_str = parts.join(" ");
                        info!("Setting fen to {fen_str}");
                        board = Board::from_fen(&fen_str);
                        println!("{board}");
                    }
                    SetSubcommand::Depth { depth } => {
                        info!("Changing search depth from {inp_depth} to {depth}");
                        search.change_depth(depth)?;
                    }
                    SetSubcommand::LogLevel { level } => {
                        let new_level: Level = level.into();
                        info!("Setting log level to {new_level}");
                        if let Err(e) = set_log_level(new_level) {
                            error!("Failed to set log level: {e:?}");
                        }
                    }
                    SetSubcommand::LogFile { enable } => {
                        let enable_bool = match enable.to_lowercase().as_str() {
                            "true" | "t" | "1" | "on" => true,
                            "false" | "f" | "0" | "off" => false,
                            _ => {
                                error!(
                                    "Invalid value for log-file: '{}'. Use 'true' or 'false'.",
                                    enable
                                );
                                continue;
                            }
                        };
                        info!("Setting file logging to: {enable_bool}");
                        if let Err(e) = toggle_file_logging(enable_bool) {
                            error!("Failed to toggle file logging: {e:?}");
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
