use std::io::{self, Write};

use eschec::{board::*, clear_screen, get_input};

#[allow(dead_code)]
fn main() -> anyhow::Result<()> {
    color_backtrace::install();
    let mut board = Board::new();

    let stdin: io::Stdin = io::stdin();
    loop {
        println!("{}", board);

        let mut s = String::new();
        print!("{} >> ", board.stm);
        io::stdout().flush()?;
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
            eprintln!("Failed to make move: {}", e);
            continue;
        }

        let computer_move = board.suggest_rand_move()?;
        println!(
            "Computed random move: {}, {}",
            computer_move.0, computer_move.1
        );

        if let Err(e) = board.make_move(computer_move.0, computer_move.1) {
            eprintln!("Failed to make move: {}", e);
            continue;
        }

        // Sleep to give some delay
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
}
