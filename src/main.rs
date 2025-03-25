use std::io::{self};

use components::Square;
use eschec::{board::*, clear_screen};

fn main() -> anyhow::Result<()> {
    let mut board = Board::new();

    let stdin: io::Stdin = io::stdin();
    loop {
        let mut s = String::new();
        stdin.read_line(&mut s).unwrap(); // Changed from read_to_string to read_line
        clear_screen()?;

        let trimmed = s.trim(); // Remove any trailing newline or spaces
        if trimmed.is_empty() {
            continue; // Ignore empty input
        }

        let (from, to) = match trimmed.split_once(' ') {
            Some((f, t)) => (f, t),
            None => {
                eprintln!("Invalid input format. Expected 'from to'.");
                continue;
            }
        };

        let from_pos: usize = match from.parse() {
            Ok(num) => num,
            Err(_) => {
                eprintln!("Invalid 'from' position: {}", from);
                continue;
            }
        };

        let to_pos: usize = match to.parse() {
            Ok(num) => num,
            Err(_) => {
                eprintln!("Invalid 'to' position: {}", to);
                continue;
            }
        };

        let from_square = match Square::new(from_pos) {
            Some(square) => square,
            None => {
                eprintln!("Invalid 'from' square: {}", from_pos);
                continue;
            }
        };

        let to_square = match Square::new(to_pos) {
            Some(square) => square,
            None => {
                eprintln!("Invalid 'to' square: {}", to_pos);
                continue;
            }
        };

        if let Err(e) = board.make_move(from_square, to_square) {
            eprintln!("Failed to make move: {}", e);
            continue;
        }

        // Sleep to give some delay
        std::thread::sleep(std::time::Duration::from_millis(300));

        println!("Black pieces bitboard:");
        println!("{}", board.positions.all_sides[1].print_bitboard());
        println!("White pieces bitboard:");
        println!("{}", board.positions.all_sides[0].print_bitboard());
    }
}
