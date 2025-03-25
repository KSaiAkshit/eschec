use eschec::board::{fen::parse_fen, Board};

fn main() {
    let _board = Board::new();
    // let rights = eschec::board::components::CastlingRights(0b1101);

    // println!("{}", rights);
    // dbg!(parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq e3 0 1").unwrap());
    let b = parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq e3 0 1").unwrap();
    let s = eschec::board::components::Square(27);
    println!("{s}");
    // dbg!(board);
}
