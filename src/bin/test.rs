#![allow(unused)]
use std::str::FromStr;

use eschec::{Board, Square, fen, moves::MoveGen, perft};

fn main() -> miette::Result<()> {
    let fen = "rbnqknbr/pppppp1p/8/6p1/8/3P4/PPP1PPPP/RBNQKNBR w KQkq g6 1 2";
    let b = Board::from_fen(fen);
    println!("{}", b);

    let moves = MoveGen::new(eschec::Piece::Bishop, b.stm, &b);
    for (i, m) in moves.attack_bb.iter().enumerate() {
        if i == 2 {
            println!("{}", m.print_bitboard());
        }
    }

    Ok(())
}
