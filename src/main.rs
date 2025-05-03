use std::io::{self, Write};

use eschec::search::Search;
use eschec::{board::*, *};
use evaluation::CompositeEvaluator;
use miette::IntoDiagnostic;
use tracing::*;

fn main() -> miette::Result<()> {
    color_backtrace::install();
    tracing_subscriber::fmt().init();

    let span = tracing::span!(Level::INFO, "main");
    let _guard = span.enter();

    tracing::info!("Hi, game starts");

    let mut board = Board::new();
    let evaluator = CompositeEvaluator::balanced();
    let mut search = Search::new(3);

    let stdin: io::Stdin = io::stdin();
    loop {
        let span = tracing::span!(Level::INFO, "loop");
        let _guard = span.enter();
        tracing::info!("Inside game loop");
        println!("{}", board);

        let mut s = String::new();
        print!("{} >> ", board.stm);
        io::stdout().flush().into_diagnostic()?;
        stdin.read_line(&mut s).unwrap();
        clear_screen()?;

        let (from_square, to_square) = match get_input(&s) {
            Ok(f) => (f.0, f.1),
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
        };

        if let Err(e) = board.try_move(from_square, to_square) {
            eprintln!("{:?}", e);
            continue;
        }

        let res = search.find_best_move(&board, &evaluator);

        let b_move = res.best_move.unwrap();
        println!(
            "Computed best move: {}, {} in {} ms",
            b_move.0,
            b_move.1,
            res.time_taken.as_millis()
        );

        let score = board.evaluate_position(&evaluator);
        println!("Score: {}", score);

        if let Err(e) = board.try_move(b_move.0, b_move.1) {
            eprintln!("{:?}", e);
            continue;
        }
    }
}
