use std::io::{self, Write};

use eschec::{board::*, *};
use evaluation::{
    material::MaterialEvaluator, mobility::MobilityEvaluator, position::PositionEvaluator,
    CompositeEvaluator,
};
use miette::IntoDiagnostic;

fn main() -> miette::Result<()> {
    color_backtrace::install();
    let mut board = Board::new();
    let mut evaluator = CompositeEvaluator::new("CompositeEvaluator");
    evaluator
        .add_evaluator(Box::new(MaterialEvaluator::new()), 0.3)
        .add_evaluator(Box::new(PositionEvaluator::new()), 0.3)
        .add_evaluator(Box::new(MobilityEvaluator::new()), 0.2);

    let stdin: io::Stdin = io::stdin();
    loop {
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

        if let Err(e) = board.make_move(from_square, to_square) {
            eprintln!("{:?}", e);
            continue;
        }

        let score = board.evaluate_position(&evaluator);
        println!("Score: {}", score);

        let computer_move = board.suggest_rand_move()?;
        println!(
            "Computed random move: {}, {}",
            computer_move.0, computer_move.1
        );

        if let Err(e) = board.make_move(computer_move.0, computer_move.1) {
            eprintln!("{:?}", e);
            continue;
        }

        // Sleep to give some delay
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
}
