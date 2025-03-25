use eschec::board::components::{Piece, Square};

fn main() {
    let k_moves = eschec::board::moves::Moves::new(Piece::Pawn);
    // dbg!(k_moves);
    let s = 55;
    let sq = Square(s);
    let bb = k_moves.attack_bb[110];
    // println!("{}", eschec::board::components::Square(20));
    // let bb = BitBoard(63);
    println!("printing bb({sq}):\n{}", bb.print_bitboard());
    // dbg!(0xFF << 1);
}
