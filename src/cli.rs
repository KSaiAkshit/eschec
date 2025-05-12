use std::io::Write;

use clap::{Parser, Subcommand};
use shell_words;
use tracing::info;

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
        #[arg(short, long)]
        fen: Option<String>,
        /// set search depth
        #[arg(short, long)]
        depth: Option<u8>,
    },

    /// Perft game with given FEN and depth, or use default fen
    Perft {
        /// FEN string for starting position
        #[arg(short, long)]
        fen: Option<String>,
        /// set search depth
        #[arg(short, long)]
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
    Move { from: String, to: String },
    Print,
    Perft { depth: u8 },
    Reset,
    Restart,
    Quit,
}
