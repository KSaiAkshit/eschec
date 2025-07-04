use clap::{Parser, Subcommand, ValueEnum};
use tracing::Level;

use crate::START_FEN;

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
    Move { from: String, to: String },

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
