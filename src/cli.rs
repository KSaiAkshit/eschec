use std::io::Write;

use clap::{Parser, Subcommand};
use shell_words;
use tracing::info;

use crate::START_FEN;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long)]
    pub comms: bool,
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

    /// Perft game with given FEN and depth, or use default fen
    Perft {
        /// FEN string for starting position
        #[arg(short, long, default_value = START_FEN)]
        fen: Option<String>,
        /// set search depth
        #[arg(short, long, default_value = "5")]
        depth: Option<u8>,
    },
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

    /// Change the AI search depth
    #[clap(visible_alias = "d")]
    Depth { depth: u8 },

    /// Show the current evaluation of the piece
    #[clap(visible_alias = "e")]
    Evaluate,

    /// Run a perft test with given depth [default: 5]
    #[clap(visible_alias = "pe")]
    Perft { depth: Option<u8> },

    /// Restart game with same fen
    #[clap(visible_alias = "r")]
    Restart,

    /// Quit game
    #[clap(visible_alias = "q")]
    Quit,
}
