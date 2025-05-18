use eschec::board::Board;
use eschec::perft::{perft_divide, run_perft_suite};
use std::env;

fn main() {
    eschec::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: perft [depth] [fen]");
        println!("    depth: Depth to search");
        println!("    fen: (optional) FEN string for position");
        return;
    }

    let depth = match args[1].parse::<u8>() {
        Ok(d) => d,
        Err(_) => {
            println!("Invalid depth: {}", args[1]);
            return;
        }
    };

    let mut board = if args.len() > 2 {
        Board::from_fen(&args[2])
    } else {
        Board::new()
    };

    if depth == 0 {
        println!("running suite");
        run_perft_suite(&mut board, 5); // Run suite up to depth 5 by default
    } else {
        println!("running divide");
        perft_divide(&mut board, depth);
    }
}
