use eschec::board::Board;

fn main() {
    let _board = Board::new();
    let rights = eschec::board::CastlingRights(0b1101);

    println!("{}", rights);
    // dbg!(rights);
}
