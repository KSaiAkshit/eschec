use eschec::board::components::Piece;

fn main() {
    let k_moves = eschec::board::moves::Moves::new(Piece::Knight);
    let bb = k_moves.attack_bb[0];
    // println!("{}", eschec::board::components::Square(52));
    // let bb = BitBoard(63);
    println!("{}", bb.print_bitboard());
    // dbg!(0xFF << 1);
}
