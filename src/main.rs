use clap::Parser;
use eschec::comms::uci;
use eschec::{board::*, *};
use tracing::{Level, span, trace};

fn main() -> miette::Result<()> {
    init();

    let span = span!(Level::DEBUG, "main");
    let _guard = span.enter();
    match cli::Cli::parse().command {
        Some(cmd) => match cmd {
            cli::Commands::Play { fen, depth } => {
                trace!("Starting game with fen: {:?}, depth: {:?}", fen, depth);
                game_loop(fen.unwrap(), depth.unwrap())?;
            }
            cli::Commands::Perft { fen, depth, divide } => {
                trace!(
                    "Running perft with fen: {:?}, depth: {:?}, divide: {:?}",
                    fen, depth, divide
                );
                let mut board = Board::from_fen(&fen.unwrap());
                println!("{}", board);
                if divide {
                    perft::perft_divide(&mut board, depth);
                } else {
                    perft::run_perft_suite(&mut board, depth);
                }
            }
            cli::Commands::Headless { proto } => {
                trace!("Running headless with protocol: {:?}", proto);
                uci::play()?;
            }
        },
        None => {
            println!("Starting default game");
        }
    }
    Ok(())
}
