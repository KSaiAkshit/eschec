use clap::Parser;
use eschec::prelude::*;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> miette::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    utils::log::init();

    let span = span!(Level::DEBUG, "main");
    let _guard = span.enter();
    match Cli::parse().command {
        Some(cmd) => match cmd {
            Commands::Play { fen, depth } => {
                trace!("Starting game with fen: {:?}, depth: {:?}", fen, depth);
                game_loop(fen.unwrap(), depth.unwrap())?;
            }
            Commands::Perft { fen, depth, divide } => {
                trace!(
                    "Running perft with fen: {:?}, depth: {:?}, divide: {:?}",
                    fen, depth, divide
                );
                let mut board = Board::from_fen(&fen.unwrap());
                println!("{board}");
                if divide {
                    perft_divide(&mut board, depth);
                } else {
                    run_perft_suite(&mut board, depth);
                }
            }
            Commands::Headless { protocol } => {
                trace!("Running headless with protocol: {:?}", protocol);
                uci::play()?;
            }
        },
        None => {
            trace!("Running headless with protocol: uci");
            uci::play()?;
        }
    }
    Ok(())
}
