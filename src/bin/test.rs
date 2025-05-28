#![allow(unused)]
use std::str::FromStr;

use eschec::{Board, Square, fen, moves::MoveGen, perft};

fn main() -> miette::Result<()> {
    let fen = "rbnqknbr/1ppppppp/8/p7/PP6/8/2PPPPPP/RBNQKNBR b KQkq b3 1 2";

    let b = Board::from_fen(fen);
    println!("{}", b);

    let moves = MoveGen::new(eschec::Piece::Pawn, b.stm, &b);
    for (i, m) in moves.attack_bb.iter().enumerate() {
        if i == 32 {
            println!("{}", m.print_bitboard());
        }
    }

    Ok(())
}
