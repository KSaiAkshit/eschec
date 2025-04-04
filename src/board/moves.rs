use crate::board::components::Square;

use super::components::{BitBoard, BoardState, Piece, Side};

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct Moves {
    pub piece: Piece,
    pub attack_bb: Vec<BitBoard>,
    state: BoardState,
}

/// First 4 are orhtogonal, rest are diagonal
///  (N, S, W, E, NW, SE, NE, SW)
pub struct Direction;
impl Direction {
    pub const NORTH: i8 = 8;
    pub const SOUTH: i8 = -8;
    pub const WEST: i8 = -1;
    pub const EAST: i8 = 1;
    pub const NORTHWEST: i8 = -7;
    pub const SOUTHEAST: i8 = 7;
    pub const NORTHEAST: i8 = 9;
    pub const SOUTHWEST: i8 = -9;

    pub const ORTHO: [i8; 4] = [Self::NORTH, Self::SOUTH, Self::WEST, Self::EAST];
    pub const DIAG: [i8; 4] = [
        Self::NORTHEAST,
        Self::SOUTHEAST,
        Self::SOUTHWEST,
        Self::NORTHWEST,
    ];
    pub const ALL: [i8; 8] = [
        Self::NORTH,
        Self::SOUTH,
        Self::WEST,
        Self::EAST,
        Self::NORTHEAST,
        Self::SOUTHEAST,
        Self::SOUTHWEST,
        Self::NORTHWEST,
    ];
}

impl Moves {
    pub fn new(piece: Piece, stm: Side, state: BoardState) -> Self {
        let mut m = Moves::default();
        m.piece = piece;
        m.state = state;
        m.attack_bb = match piece {
            Piece::Knight => m.gen_knight_moves(stm),
            Piece::Rook => m.gen_rook_moves(stm),
            Piece::Bishop => m.gen_bishop_moves(stm),
            Piece::Queen => m.gen_queen_moves(stm),
            Piece::Pawn => m.gen_pawn_moves(stm),
            Piece::King => m.gen_king_moves(stm),
        };

        m
    }

    /// Generate all possible moves for all pieces and all sides
    /// Does contain pseudo legal moves.
    pub fn all_possible_moves() -> Vec<Self> {
        let mut moves: Vec<Moves> = Vec::new();
        Piece::all()
            .for_each(|(piece, stm)| moves.push(Moves::new(piece, stm, BoardState::default())));

        moves
    }

    pub fn make_legal(&mut self, stm: &Side, board: &BoardState) {
        let own_pieces = board.all_sides[stm.index()];

        self.attack_bb.iter_mut().for_each(|b| *b &= !own_pieces);
    }

    fn gen_pawn_moves(&self, stm: Side) -> Vec<BitBoard> {
        match stm {
            Side::White => self.gen_white_pawn_moves(),
            Side::Black => self.gen_black_pawn_moves(),
        }
    }

    fn gen_white_pawn_moves(&self) -> Vec<BitBoard> {
        let mut attack_bb = vec![BitBoard(0); 64];
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            let mut white_pawn_moves = BitBoard(0);
            let (file, _rank) = square.coords();
            if file == 0 {
                attack_bb[index] = BitBoard(0);
            }
            if file == 1 {
                for &offset in &[8, 16] {
                    let target_index = index + offset;
                    let target_bb = BitBoard(1 << target_index);
                    white_pawn_moves = white_pawn_moves | target_bb;
                }
                attack_bb[index] = white_pawn_moves;
            } else {
                let target_index = index + 8;
                if target_index < 64 {
                    let target_bb = BitBoard(1 << target_index);
                    white_pawn_moves = white_pawn_moves | target_bb;
                    attack_bb[index] = white_pawn_moves;
                }
            }
        });
        attack_bb
    }

    fn gen_black_pawn_moves(&self) -> Vec<BitBoard> {
        let mut attack_bb = vec![BitBoard(0); 64];
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            let mut black_pawn_moves = BitBoard(0);
            let (file, _rank) = square.coords();
            if file == 7 || file == 0 {
                attack_bb[index] = BitBoard(0);
            }
            if file == 6 {
                for &offset in &[8, 16] {
                    let target_index = index as i8 - offset;
                    if target_index.is_positive() {
                        let target_bb = BitBoard(1 << target_index);
                        black_pawn_moves = black_pawn_moves | target_bb;
                    }
                }
                attack_bb[index] = black_pawn_moves;
            } else {
                let target_index = index as i8 - 8;
                if (8..48).contains(&target_index) {
                    let target_bb = BitBoard(1 << target_index);
                    black_pawn_moves = black_pawn_moves | target_bb;
                    attack_bb[index] = black_pawn_moves;
                }
            }
        });
        attack_bb
    }

    fn gen_queen_moves(&self, stm: Side) -> Vec<BitBoard> {
        let mut attack_bb = vec![BitBoard(0); 64];
        let bishop_moves = self.gen_bishop_moves(stm);
        let rook_moves = self.gen_rook_moves(stm);
        (0..64).for_each(|index| {
            attack_bb[index] = bishop_moves[index] | rook_moves[index];
        });
        attack_bb
    }

    fn gen_bishop_moves(&self, stm: Side) -> Vec<BitBoard> {
        let mut attack_bb = vec![BitBoard(0); 64];
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            let mut bishop_moves = BitBoard(0);
            let (file, rank) = square.coords();
            for &delta in &Direction::DIAG {
                let mut target_index = index as i8 + delta;
                while (0..64).contains(&target_index) {
                    let target_square =
                        Square::new(target_index as usize).expect("get a valid index");
                    if self.state.all_sides[stm.index()].contains_square(target_square.index()) {
                        break;
                    }
                    let (target_file, target_rank) = target_square.coords();
                    let file_diff = file as i8 - target_file as i8;
                    let rank_diff = rank as i8 - target_rank as i8;
                    if file_diff.abs() == rank_diff.abs() {
                        let target_bb = BitBoard(1 << target_index);
                        bishop_moves = bishop_moves | target_bb;
                    }
                    if file_diff.abs() != rank_diff.abs() || file_diff == 0 {
                        break;
                    }
                    target_index += delta;
                }
            }
            attack_bb[index] = bishop_moves;
        });
        attack_bb
    }

    fn gen_rook_moves(&self, stm: Side) -> Vec<BitBoard> {
        let mut attack_bb = vec![BitBoard(0); 64];
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            let mut rook_moves = BitBoard(0);
            let (file, rank) = square.coords();
            for &delta in &Direction::ORTHO {
                let mut target_index = index as i8 + delta;
                while (0..64).contains(&target_index) {
                    let target_square =
                        Square::new(target_index as usize).expect("get a valid index");
                    if self.state.all_sides[stm.index()].contains_square(target_square.index()) {
                        break;
                    }
                    let (target_file, target_rank) = target_square.coords();
                    if target_file != file && target_rank == rank {
                        let target_bb = BitBoard(1 << target_index);
                        rook_moves = rook_moves | target_bb;
                    }
                    if target_rank != rank && target_file == file {
                        let target_bb = BitBoard(1 << target_index);
                        rook_moves = rook_moves | target_bb;
                    }
                    target_index += delta;
                }
            }
            attack_bb[index] = rook_moves;
        });
        attack_bb
    }

    fn gen_king_moves(&self, _stm: Side) -> Vec<BitBoard> {
        let mut attack_bb = vec![BitBoard(0); 64];
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            let mut king_moves = BitBoard(0);
            let (file, _rank) = square.coords();

            for delta in &Direction::DIAG {
                let target_index = index as i8 + delta;
                if target_index < 64 && 0 < target_index {
                    let target_bb = BitBoard(1 << target_index);
                    king_moves = king_moves | target_bb;
                }
            }

            for &f in &Direction::ORTHO {
                if f != file as i8 {
                    let target_index = index as i8 + f;
                    if target_index < 64 && 0 < target_index {
                        let target_bb = BitBoard(1 << target_index);
                        king_moves = king_moves | target_bb;
                    }
                }
            }
            attack_bb[index] = king_moves;
        });
        attack_bb
    }

    fn gen_knight_moves(&self, _stm: Side) -> Vec<BitBoard> {
        let mut attack_bb = vec![BitBoard(0); 64];
        let knight_offsets: [i8; 8] = [-17, -15, -10, -6, 6, 10, 15, 17];

        (0..64).for_each(|index| {
            let mut knight_moves = BitBoard(0);
            let square = Square::new(index).expect("get a valid index").coords();
            let (file, rank) = square;

            for &offset in knight_offsets.iter() {
                let target_index = index as i8 + offset;
                if (0..64).contains(&target_index) {
                    let target_square_y = target_index as usize / 8;
                    let target_square_x = target_index as usize % 8;

                    // Calculate the maximum coordinate move distance
                    let max_coord_move_dst = i8::max(
                        (rank as i8 - target_square_x as i8).abs(),
                        (file as i8 - target_square_y as i8).abs(),
                    );

                    // If the maximum coordinate move distance is 2, the move is valid
                    if max_coord_move_dst == 2 {
                        let target_bb = BitBoard(1 << target_index);

                        // Add the target square bitboard to knight_moves
                        knight_moves = knight_moves | target_bb;
                    }
                }
            }
            attack_bb[index] = knight_moves;
        });

        attack_bb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pawn_moves_white() {
        let stm = Side::White;
        let pawn = Piece::Pawn;
        let state = BoardState::default();
        let moves = Moves::new(pawn, stm, state);

        let square_index = 8;
        let expected = BitBoard(1 << 16) | BitBoard(1 << 24);
        assert_eq!(moves.attack_bb[square_index], expected);
    }

    #[test]
    fn test_pawn_moves_black() {
        let stm = Side::Black;
        let pawn = Piece::Pawn;
        let state = BoardState::default();
        let moves = Moves::new(pawn, stm, state);

        let square_index = 48;
        let expected = BitBoard(1 << 40) | BitBoard(1 << 32);
        assert_eq!(moves.attack_bb[square_index], expected);
    }

    #[test]
    fn test_knight_moves() {
        let stm = Side::White;
        let knight = Piece::Knight;
        let state = BoardState::default();
        let moves = Moves::new(knight, stm, state);

        let square = Square::new(1).unwrap(); // A2 square
        let expected_moves = BitBoard(1 << 18) | BitBoard(1 << 16) | BitBoard(1 << 11); // B1, C3, and A3 are valid moves
        assert_eq!(moves.attack_bb[square.index()], expected_moves);
    }

    #[test]
    fn test_rook_moves() {
        let stm = Side::White;
        let rook = Piece::Rook;
        let state = BoardState::default();
        let moves = Moves::new(rook, stm, state);

        let square_index = 0; // A1 square
        let mut expected_moves = (0..8)
            .map(|r| BitBoard(1 << r) | BitBoard(1 << (r * 8)))
            .fold(BitBoard(0), |acc, bb| acc | bb);
        expected_moves.capture(0);
        assert_eq!(moves.attack_bb[square_index], expected_moves);
    }

    #[test]
    fn test_bishop_moves() {
        let stm = Side::White;
        let bishop = Piece::Bishop;
        let state = BoardState::default();
        let moves = Moves::new(bishop, stm, state);

        let square_index = 18; // C3 square
        let expected_moves = BitBoard(9241421692918565393);

        println!("expected: \n{}", expected_moves.print_bitboard());
        assert_eq!(moves.attack_bb[square_index], expected_moves);
    }

    #[test]
    fn test_queen_moves() {
        let stm = Side::White;
        let queen = Piece::Queen;
        let state = BoardState::default();
        let moves = Moves::new(queen, stm, state);

        let square_index = 0; // A1 square
        let expected_moves =
            moves.gen_rook_moves(stm)[square_index] | moves.gen_bishop_moves(stm)[square_index];
        assert_eq!(moves.attack_bb[square_index], expected_moves);
    }

    #[test]
    fn test_king_moves() {
        let stm = Side::White;
        let king = Piece::King;
        let state = BoardState::default();
        let moves = Moves::new(king, stm, state);

        let square_index = 36; // E4 square
        let expected_moves = BitBoard(1 << 27)
            | BitBoard(1 << 28)
            | BitBoard(1 << 29)
            | BitBoard(1 << 35)
            | BitBoard(1 << 37)
            | BitBoard(1 << 43)
            | BitBoard(1 << 44)
            | BitBoard(1 << 45);
        assert_eq!(moves.attack_bb[square_index], expected_moves);
    }

    #[test]
    fn test_all_possible_moves() {
        let moves = Moves::all_possible_moves();

        // Ensure the correct number of moves are generated
        assert_eq!(moves.len(), 12);
    }

    #[test]
    fn test_piece_move_generation() {
        let m = Moves::default();
        let white_pawn_moves = Moves::gen_pawn_moves(&m, Side::White);
        let black_pawn_moves = Moves::gen_pawn_moves(&m, Side::Black);
        let knight_moves = Moves::gen_knight_moves(&m, Side::White);
        let rook_moves = Moves::gen_rook_moves(&m, Side::White);
        let bishop_moves = Moves::gen_bishop_moves(&m, Side::White);
        let queen_moves = Moves::gen_queen_moves(&m, Side::White);
        let king_moves = Moves::gen_king_moves(&m, Side::White);

        assert!(!white_pawn_moves.is_empty());
        assert!(!black_pawn_moves.is_empty());
        assert!(!knight_moves.is_empty());
        assert!(!rook_moves.is_empty());
        assert!(!bishop_moves.is_empty());
        assert!(!queen_moves.is_empty());
        assert!(!king_moves.is_empty());
    }
}
