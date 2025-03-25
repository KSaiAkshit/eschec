use eschec::board::components::{Piece, Square};

fn main() {
    let k_moves = eschec::board::moves::Moves::new(Piece::Bishop);
    let s = 50;
    let sq = Square(s);
    let bb = k_moves.attack_bb[s];
    // println!("{}", eschec::board::components::Square(20));
    // let bb = BitBoard(63);
    println!("printing bb({sq}):\n{}", bb.print_bitboard());
    // dbg!(0xFF << 1);
}
