use std::io::{self, Write};

use components::{Side, Square};
use eschec::{board::*, clear_screen, get_input};
use moves::Moves;

fn main2() {
    let mut board = Board::from_fen("8/3p3p/8/8/8/8/3P3P/8 w KQkq - 0 1");
    let _ = board.make_move(Square::new(10).unwrap(), Square::new(18).unwrap());
    let mut m = Moves::new(components::Piece::Rook, Side::White, board.positions);
    println!("{}", board);
    m.make_legal(&Side::White, &board.positions);
    let mut idx = 0;
    for b in m.attack_bb {
        idx += 1;
        println!("{idx}: \n{}", b.print_bitboard())
    }
}

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

        // Sleep to give some delay
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
}
