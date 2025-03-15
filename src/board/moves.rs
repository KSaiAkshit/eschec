use crate::board::components::Square;

use super::components::{BitBoard, Piece, Side};

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct Moves {
    pub piece: Piece,
    pub attack_bb: Vec<BitBoard>,
}

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
}

impl Moves {
    /// First 4 are orhtogonal, rest are diagonal
    ///                                (N, S, W, E, NW, SE, NE, SW)
    pub fn new(piece: Piece, stm: Side) -> Self {
        match piece {
            Piece::Knight => Self::gen_knight_moves(stm),
            Piece::Rook => Self::gen_rook_moves(stm),
            Piece::Bishop => Self::gen_bishop_moves(stm),
            Piece::Queen => Self::gen_queen_moves(stm),
            Piece::Pawn => Self::gen_pawn_moves(stm),
            Piece::King => Self::gen_king_moves(stm),
        }
    }

    pub fn all_legal_moves() -> Vec<Self> {
        let mut moves: Vec<Moves> = Vec::new();
        Piece::all().for_each(|(piece, stm)| moves.push(Moves::new(piece, stm)));

        moves
    }

    pub fn gen_pawn_moves(stm: Side) -> Self {
        let attack_bb = match stm {
            Side::White => Self::gen_white_pawn_moves(),
            Side::Black => Self::gen_black_pawn_moves(),
        };

        Self {
            piece: Piece::Pawn,
            attack_bb,
        }
    }

    fn gen_white_pawn_moves() -> Vec<BitBoard> {
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

    fn gen_black_pawn_moves() -> Vec<BitBoard> {
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

    pub fn gen_queen_moves(stm: Side) -> Self {
        let mut attack_bb = vec![BitBoard(0); 64];
        let bishop_moves = Self::gen_bishop_moves(stm).attack_bb;
        let rook_moves = Self::gen_rook_moves(stm).attack_bb;
        (0..64).for_each(|index| {
            // let queen_moves: BitBoard = BitBoard(0);
            attack_bb[index] = bishop_moves[index] | rook_moves[index];
        });
        Self {
            piece: Piece::Queen,
            attack_bb,
        }
    }

    pub fn gen_bishop_moves(_stm: Side) -> Self {
        let mut attack_bb = vec![BitBoard(0); 64];
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            let mut bishop_moves = BitBoard(0);
            let (file, rank) = square.coords();
            // diagonal = [+9, -9]
            for &delta in &[-9, -7, 7, 9] {
                let mut target_index = index as i8 + delta;
                while (0..64).contains(&target_index) {
                    let target_square =
                        Square::new(target_index as usize).expect("get a valid index");
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
        Self {
            piece: Piece::Bishop,
            attack_bb,
        }
    }

    pub fn gen_rook_moves(_stm: Side) -> Self {
        let mut attack_bb = vec![BitBoard(0); 64];
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            let mut rook_moves = BitBoard(0);
            let (file, rank) = square.coords();

            // Vertical
            for f in 0..8 {
                if f != file {
                    let target_index = f * 8 + rank;
                    let target_bb = BitBoard(1 << target_index);
                    rook_moves = rook_moves | target_bb;
                }
            }

            // Horizontal
            for r in 0..8 {
                if r != rank {
                    let target_index = r + file * 8;
                    let target_bb = BitBoard(1 << target_index);
                    rook_moves = rook_moves | target_bb;
                }
            }
            attack_bb[index] = rook_moves;
        });
        Self {
            piece: Piece::Rook,
            attack_bb,
        }
    }

    pub fn gen_king_moves(_stm: Side) -> Self {
        let mut attack_bb = vec![BitBoard(0); 64];
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            let mut king_moves = BitBoard(0);
            let (file, _rank) = square.coords();

            for delta in [-9, -7, 7, 9] {
                let target_index = index as i8 + delta;
                if target_index < 64 && 0 < target_index {
                    let target_bb = BitBoard(1 << target_index);
                    king_moves = king_moves | target_bb;
                }
            }

            for f in [1, 8, -8i8, -1i8] {
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
        Self {
            piece: Piece::King,
            attack_bb,
        }
    }

    pub fn gen_knight_moves(_stm: Side) -> Self {
        let mut attack_bb = vec![BitBoard(0); 64];
        let knight_offsets: [i8; 8] = [-17, -15, -10, -6, 6, 10, 15, 17];

        (0..64).for_each(|index| {
            let mut knight_moves = BitBoard(0);

            // Iterate over each knight offset
            for &offset in knight_offsets.iter() {
                let target_index = index as i8 + offset;
                if (0..64).contains(&target_index) {
                    // Calculate the coordinates of the knight square and target square
                    let knight_square_y = index / 8;
                    let knight_square_x = index % 8;
                    let target_square_y = target_index as usize / 8;
                    let target_square_x = target_index as usize % 8;

                    // Calculate the maximum coordinate move distance
                    let max_coord_move_dst = i8::max(
                        (knight_square_x as i8 - target_square_x as i8).abs(),
                        (knight_square_y as i8 - target_square_y as i8).abs(),
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

        Moves {
            piece: Piece::Knight,
            attack_bb,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pawn_moves_white() {
        let stm = Side::White;
        let moves = Moves::gen_pawn_moves(stm);

        let square_index = 8;
        let expected = BitBoard(1 << 16) | BitBoard(1 << 24);
        assert_eq!(moves.attack_bb[square_index], expected);
    }

    #[test]
    fn test_pawn_moves_black() {
        let stm = Side::Black;
        let moves = Moves::gen_pawn_moves(stm);

        let square_index = 48;
        let expected = BitBoard(1 << 40) | BitBoard(1 << 32);
        assert_eq!(moves.attack_bb[square_index], expected);
    }

    #[test]
    fn test_knight_moves() {
        let stm = Side::White;
        let moves = Moves::gen_knight_moves(stm);

        let square = Square::new(1).unwrap(); // A2 square
        let expected_moves = BitBoard(1 << 18) | BitBoard(1 << 16) | BitBoard(1 << 11); // B1, C3, and A3 are valid moves
        assert_eq!(moves.attack_bb[square.index()], expected_moves);
    }

    #[test]
    fn test_rook_moves() {
        let stm = Side::White;
        let moves = Moves::gen_rook_moves(stm);

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
        let moves = Moves::gen_bishop_moves(stm);

        let square_index = 18; // C3 square
        let expected_moves = BitBoard(9241421692918565393);

        println!("expected: \n{}", expected_moves.print_bitboard());
        assert_eq!(moves.attack_bb[square_index], expected_moves);
    }

    #[test]
    fn test_queen_moves() {
        let stm = Side::White;
        let moves = Moves::gen_queen_moves(stm);

        let square_index = 0; // A1 square
        let expected_moves = Moves::gen_rook_moves(stm).attack_bb[square_index]
            | Moves::gen_bishop_moves(stm).attack_bb[square_index];
        assert_eq!(moves.attack_bb[square_index], expected_moves);
    }

    #[test]
    fn test_king_moves() {
        let stm = Side::White;
        let moves = Moves::gen_king_moves(stm);

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
    fn test_all_legal_moves() {
        let moves = Moves::all_legal_moves();

        // Ensure the correct number of moves are generated
        assert_eq!(moves.len(), 12);
    }

    #[test]
    fn test_piece_move_generation() {
        let white_pawn_moves = Moves::gen_pawn_moves(Side::White);
        let black_pawn_moves = Moves::gen_pawn_moves(Side::Black);
        let knight_moves = Moves::gen_knight_moves(Side::White);
        let rook_moves = Moves::gen_rook_moves(Side::White);
        let bishop_moves = Moves::gen_bishop_moves(Side::White);
        let queen_moves = Moves::gen_queen_moves(Side::White);
        let king_moves = Moves::gen_king_moves(Side::White);

        assert!(!white_pawn_moves.attack_bb.is_empty());
        assert!(!black_pawn_moves.attack_bb.is_empty());
        assert!(!knight_moves.attack_bb.is_empty());
        assert!(!rook_moves.attack_bb.is_empty());
        assert!(!bishop_moves.attack_bb.is_empty());
        assert!(!queen_moves.attack_bb.is_empty());
        assert!(!king_moves.attack_bb.is_empty());
    }
}
