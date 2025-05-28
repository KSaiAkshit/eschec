use crate::{CastlingRights, board::components::Square};

use super::{
    Board,
    components::{BitBoard, BoardState, Piece, Side},
};

pub mod move_info;
pub mod precomputed;

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct MoveGen {
    pub piece: Piece,
    pub attack_bb: Vec<BitBoard>,
    state: BoardState,
    ep_square: Option<Square>,
    castling: CastlingRights,
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

impl MoveGen {
    pub fn new(piece: Piece, stm: Side, state: &Board) -> Self {
        let mut moves = MoveGen::default();
        moves.piece = piece;
        moves.state = state.positions;
        moves.ep_square = state.enpassant_square;
        moves.castling = state.castling_rights;
        moves.attack_bb = match piece {
            Piece::Knight => moves.gen_knight_moves(stm),
            Piece::Rook => moves.gen_rook_moves(stm),
            Piece::Bishop => moves.gen_bishop_moves(stm),
            Piece::Queen => moves.gen_queen_moves(stm),
            Piece::Pawn => moves.gen_pawn_moves(stm),
            Piece::King => moves.gen_king_moves(stm),
        };

        moves
    }

    /// Generate all possible moves for all pieces and all sides
    /// Does contain pseudo legal moves.
    pub fn all_possible_moves(state: Board) -> Vec<Self> {
        let mut moves: Vec<MoveGen> = Vec::new();
        Piece::all().for_each(|(piece, stm)| moves.push(MoveGen::new(piece, stm, &state)));

        moves
    }

    #[deprecated(note = "Moves generated are already pseudo-legal. This doesn't acheive anything")]
    pub fn make_legal(&mut self, stm: &Side, board: &BoardState) {
        let own_pieces = board.get_side_bb(stm);

        self.attack_bb.iter_mut().for_each(|b| *b &= !*own_pieces);
    }

    fn gen_pawn_moves(&self, stm: Side) -> Vec<BitBoard> {
        match stm {
            Side::White => self.gen_white_pawn_moves(),
            Side::Black => self.gen_black_pawn_moves(),
        }
    }

    fn gen_white_pawn_moves(&self) -> Vec<BitBoard> {
        let mut attack_bb = vec![BitBoard(0); 64];
        let enemy_pieces = self.state.get_side_bb(&Side::Black);
        let ally_pieces = self.state.get_side_bb(&Side::White);
        let all_pieces = ally_pieces | enemy_pieces;
        (0..64).for_each(|index| {
            let mut white_pawn_moves = BitBoard(0);
            let square = Square::new(index).expect("Get a valid index");
            let (rank, file) = square.coords();

            if rank == 7 {
                return; // 'continue' equivalent in closures;
            }

            let fwd = index + 8;
            if fwd < 64 && !all_pieces.contains_square(fwd) {
                white_pawn_moves.set(fwd);

                if rank == 1 && !all_pieces.contains_square(fwd + 8) {
                    white_pawn_moves.set(fwd + 8);
                }
            }

            // Cannot be on leftmost file and still capture to the left
            if file > 0 {
                let cap_left = index + 7;
                if cap_left < 64
                    && (enemy_pieces.contains_square(cap_left)
                        || (self.ep_square.is_some_and(|ep| ep.index() == cap_left)))
                {
                    white_pawn_moves.set(cap_left);
                }
            }

            // Cannot be on rightmost file and still capture to the right
            if file < 7 {
                let cap_right = index + 9;
                if cap_right < 64
                    && (enemy_pieces.contains_square(cap_right)
                        || self.ep_square.is_some_and(|ep| ep.index() == cap_right))
                {
                    white_pawn_moves.set(cap_right);
                }
            }
            attack_bb[index] = white_pawn_moves;
        });
        attack_bb
    }

    pub fn gen_black_pawn_moves(&self) -> Vec<BitBoard> {
        let mut attack_bb = vec![BitBoard(0); 64];
        let enemy_pieces = self.state.get_side_bb(&Side::White);
        let ally_pieces = self.state.get_side_bb(&Side::Black);
        let all_pieces = ally_pieces | enemy_pieces;
        (0..64).for_each(|index| {
            let mut black_pawn_moves = BitBoard(0);
            let square = Square::new(index).expect("Get a valid index");
            let (rank, file) = square.coords();

            if rank == 0 {
                return; // 'continue' equivalent in closures;
            }

            let fwd = index - 8;
            if fwd >= 8 && !all_pieces.contains_square(fwd) {
                black_pawn_moves.set(fwd);

                if rank == 6 && !all_pieces.contains_square(fwd - 8) {
                    black_pawn_moves.set(fwd - 8);
                }
            }

            // Cannot be on leftmost file and still capture to the left
            if file > 0 {
                let cap_left = index - 9;
                if cap_left < 64
                    && (enemy_pieces.contains_square(cap_left)
                        || self.ep_square.is_some_and(|ep| ep.index() == cap_left))
                {
                    black_pawn_moves.set(cap_left);
                }
            }

            // Cannot be on rightmost file and still capture to the right
            if file < 7 {
                let cap_right = index - 7;
                if cap_right < 64
                    && (enemy_pieces.contains_square(cap_right)
                        || self.ep_square.is_some_and(|ep| ep.index() == cap_right))
                {
                    black_pawn_moves.set(cap_right);
                }
            }
            attack_bb[index] = black_pawn_moves;
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
        let ally_pieces = self.state.get_side_bb(&stm);
        let enemy_pieces = self.state.get_side_bb(&stm.flip());

        (0..64).for_each(|index| {
            let mut bishop_moves = BitBoard(0);
            let square = Square::new(index).expect("Get a valid index");
            let (rank, file) = square.coords();
            for &delta in &Direction::DIAG {
                let mut target_index = index as i8 + delta;
                while (0..64).contains(&target_index) {
                    let target_square =
                        Square::new(target_index as usize).expect("get a valid index");
                    // Contains an ally, do not add to possible moves
                    if ally_pieces.contains_square(target_square.index()) {
                        break;
                    }
                    let (target_rank, target_file) = target_square.coords();
                    let file_diff = rank as i8 - target_rank as i8;
                    let rank_diff = file as i8 - target_file as i8;
                    if file_diff.abs() == rank_diff.abs() {
                        let target_bb = BitBoard(1 << target_index);
                        bishop_moves = bishop_moves | target_bb;
                    }
                    // Contains an enemy, add the capture to possible moves
                    if enemy_pieces.contains_square(target_square.index()) {
                        break;
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
        let ally_pieces = self.state.get_side_bb(&stm);
        let enemy_pieces = self.state.get_side_bb(&stm.flip());
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            let mut rook_moves = BitBoard(0);
            let (rank, file) = square.coords();
            for &delta in &Direction::ORTHO {
                let mut target_index = index as i8;
                loop {
                    target_index += delta;
                    if !(0..64).contains(&target_index) {
                        break;
                    }
                    let target_square =
                        Square::new(target_index as usize).expect("get a valid index");
                    let (target_rank, target_file) = target_square.coords();

                    if (delta == Direction::EAST || delta == Direction::WEST) && target_rank != rank
                    {
                        break;
                    }
                    if (delta == Direction::NORTH || delta == Direction::SOUTH)
                        && target_file != file
                    {
                        break;
                    }
                    if ally_pieces.contains_square(target_square.index()) {
                        break;
                    }
                    rook_moves.set(target_index as usize);
                    if enemy_pieces.contains_square(target_square.index()) {
                        break;
                    }
                    // target_index += delta;
                }
            }
            attack_bb[index] = rook_moves;
        });
        attack_bb
    }

    fn gen_king_moves(&self, stm: Side) -> Vec<BitBoard> {
        let mut attack_bb = vec![BitBoard(0); 64];
        let ally_pieces = self.state.get_side_bb(&stm);
        let enemy_pieces = self.state.get_side_bb(&stm.flip());
        let all_pieces = ally_pieces | enemy_pieces;
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            let mut king_moves = BitBoard(0);
            let (rank, file) = square.coords();

            for &delta in &Direction::ALL {
                let target_index = index as i8 + delta;
                if (0..64).contains(&target_index) {
                    let target_square =
                        Square::new(target_index as usize).expect("Should get a valid index");
                    let (target_rank, target_file) = target_square.coords();

                    if (file as i8 - target_file as i8).abs() <= 1
                        && (rank as i8 - target_rank as i8) <= 1
                        && !ally_pieces.contains_square(target_square.index())
                    {
                        king_moves.set(target_index as usize);
                    }
                }
            }

            // white king on e1
            if stm == Side::White && index == 4 {
                if self
                    .castling
                    .allows(CastlingRights(CastlingRights::WHITE_00))
                    && !all_pieces.contains_square(5)
                    && !all_pieces.contains_square(6)
                    && !self.is_square_attacked(4, stm.flip())
                    && !self.is_square_attacked(5, stm.flip())
                    && !self.is_square_attacked(6, stm.flip())
                {
                    king_moves.set(6); // g1
                }
                if self
                    .castling
                    .allows(CastlingRights(CastlingRights::WHITE_000))
                    && !all_pieces.contains_square(2)
                    && !all_pieces.contains_square(3)
                    && !self.is_square_attacked(4, stm.flip())
                    && !self.is_square_attacked(3, stm.flip())
                    && !self.is_square_attacked(2, stm.flip())
                {
                    king_moves.set(2); // c1
                }
            }
            // Black king on e8
            else if stm == Side::Black && index == 60 {
                if self
                    .castling
                    .allows(CastlingRights(CastlingRights::BLACK_00))
                    && !all_pieces.contains_square(5)
                    && !all_pieces.contains_square(6)
                    && !self.is_square_attacked(60, stm.flip())
                    && !self.is_square_attacked(61, stm.flip())
                    && !self.is_square_attacked(62, stm.flip())
                {
                    king_moves.set(62); // g8
                }
                if self
                    .castling
                    .allows(CastlingRights(CastlingRights::BLACK_000))
                    && !all_pieces.contains_square(2)
                    && !all_pieces.contains_square(3)
                    && !self.is_square_attacked(60, stm.flip())
                    && !self.is_square_attacked(59, stm.flip())
                    && !self.is_square_attacked(58, stm.flip())
                {
                    king_moves.set(58); // c8
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
            let (rank, file) = square;

            for &offset in knight_offsets.iter() {
                let target_index = index as i8 + offset;
                if (0..64).contains(&target_index) {
                    let target_square_y = target_index as usize / 8;
                    let target_square_x = target_index as usize % 8;

                    // Calculate the maximum coordinate move distance
                    let max_coord_move_dst = i8::max(
                        (file as i8 - target_square_x as i8).abs(),
                        (rank as i8 - target_square_y as i8).abs(),
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

    fn is_square_attacked(&self, square_index: usize, by_side: Side) -> bool {
        let pawn_attackw = if by_side == Side::White {
            // White pawns attack diagonally up
            let up_left = square_index.wrapping_sub(9);
            let up_right = square_index.wrapping_sub(7);

            let is_not_leftmost = square_index % 8 != 0;
            let is_not_rightmost = square_index % 8 != 0;

            let pawn_bb = self.state.get_piece_bb(&by_side, &Piece::Pawn);

            let attacked_by_left =
                is_not_leftmost && square_index > 8 && pawn_bb.contains_square(up_left);
            let attacked_by_right =
                is_not_rightmost && square_index > 8 && pawn_bb.contains_square(up_right);
            attacked_by_left || attacked_by_right
        } else {
            // Black pawns attack diagonally down-left and down-right
            let down_left = square_index + 7;
            let down_right = square_index + 9;

            let is_not_leftmost = square_index % 8 != 0;
            let is_not_rightmost = square_index % 8 != 7;

            let pawn_bb = self.state.get_piece_bb(&by_side, &Piece::Pawn);

            let attacked_by_left =
                is_not_rightmost && square_index < 56 && pawn_bb.contains_square(down_left);
            let attacked_by_right =
                is_not_leftmost && square_index < 56 && pawn_bb.contains_square(down_right);

            attacked_by_left || attacked_by_right
        };

        if pawn_attackw {
            return true;
        }

        // Check if square is attacked by knights
        let knight_moves = MoveGen::new(
            Piece::Knight,
            by_side,
            &Board {
                positions: self.state,
                ..Default::default()
            },
        );
        let knight_bb = self.state.get_piece_bb(&by_side, &Piece::Knight);

        for knight_idx in knight_bb.iter_bits() {
            if knight_moves.attack_bb[knight_idx].contains_square(square_index) {
                return true;
            }
        }

        // Check if square is attacked by bishops or queens (diagonal attacks)
        let bishop_moves = MoveGen::new(
            Piece::Bishop,
            by_side,
            &Board {
                positions: self.state,
                ..Default::default()
            },
        );
        let bishop_bb = self.state.get_piece_bb(&by_side, &Piece::Bishop);
        let queen_bb = self.state.get_piece_bb(&by_side, &Piece::Queen);

        for bishop_idx in bishop_bb.iter_bits().chain(queen_bb.iter_bits()) {
            if bishop_moves.attack_bb[bishop_idx].contains_square(square_index) {
                return true;
            }
        }

        // Check if square is attacked by rooks or queens (orthogonal attacks)
        let rook_moves = MoveGen::new(
            Piece::Rook,
            by_side,
            &Board {
                positions: self.state,
                ..Default::default()
            },
        );
        let rook_bb = self.state.get_piece_bb(&by_side, &Piece::Rook);

        for rook_idx in rook_bb.iter_bits().chain(queen_bb.iter_bits()) {
            if rook_moves.attack_bb[rook_idx].contains_square(square_index) {
                return true;
            }
        }

        // Check if square is attacked by king (adjacent squares)
        let king_moves = MoveGen::new(
            Piece::King,
            by_side,
            &Board {
                positions: self.state,
                ..Default::default()
            },
        );
        let king_bb = self.state.get_piece_bb(&by_side, &Piece::King);

        for king_idx in king_bb.iter_bits() {
            if king_moves.attack_bb[king_idx].contains_square(square_index) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::init;

    use super::*;

    #[test]
    fn test_pawn_moves_white() {
        init();
        let stm = Side::White;
        let pawn = Piece::Pawn;
        let state = Board::default();
        let moves = MoveGen::new(pawn, stm, &state);

        let square_index = 8;
        let expected = BitBoard(1 << 16) | BitBoard(1 << 24);
        assert_eq!(moves.attack_bb[square_index], expected);
    }

    #[test]
    fn test_pawn_moves_black() {
        init();
        let stm = Side::Black;
        let pawn = Piece::Pawn;
        let state = Board::default();
        let moves = MoveGen::new(pawn, stm, &state);

        let square_index = 48;
        let expected = BitBoard(1 << 40) | BitBoard(1 << 32);
        assert_eq!(moves.attack_bb[square_index], expected);
    }

    #[test]
    fn test_knight_moves() {
        init();
        let stm = Side::White;
        let knight = Piece::Knight;
        let state = Board::default();
        let moves = MoveGen::new(knight, stm, &state);

        let square = Square::new(1).unwrap(); // A2 square
        let expected_moves = BitBoard(1 << 18) | BitBoard(1 << 16) | BitBoard(1 << 11); // B1, C3, and A3 are valid moves
        assert_eq!(moves.attack_bb[square.index()], expected_moves);
    }

    #[test]
    fn test_rook_moves() {
        init();
        let stm = Side::White;
        let rook = Piece::Rook;
        let state = Board::default();
        let moves = MoveGen::new(rook, stm, &state);

        let square_index = 0; // A1 square
        let mut expected_moves = (0..8)
            .map(|r| BitBoard(1 << r) | BitBoard(1 << (r * 8)))
            .fold(BitBoard(0), |acc, bb| acc | bb);
        expected_moves.capture(0);
        assert_eq!(moves.attack_bb[square_index], expected_moves);
    }

    #[test]
    fn test_bishop_moves() {
        init();
        let stm = Side::White;
        let bishop = Piece::Bishop;
        let state = Board::default();
        let moves = MoveGen::new(bishop, stm, &state);

        let square_index = 18; // C3 square
        let expected_moves = BitBoard(9241421692918565393);

        println!("expected: \n{}", expected_moves.print_bitboard());
        assert_eq!(moves.attack_bb[square_index], expected_moves);
    }

    #[test]
    fn test_queen_moves() {
        init();
        let stm = Side::White;
        let queen = Piece::Queen;
        let state = Board::default();
        let moves = MoveGen::new(queen, stm, &state);

        let square_index = 0; // A1 square
        let expected_moves =
            moves.gen_rook_moves(stm)[square_index] | moves.gen_bishop_moves(stm)[square_index];
        assert_eq!(moves.attack_bb[square_index], expected_moves);
    }

    #[test]
    fn test_king_moves() {
        init();
        let stm = Side::White;
        let king = Piece::King;
        let state = Board::default();
        let moves = MoveGen::new(king, stm, &state);

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
        init();
        let moves = MoveGen::all_possible_moves(Board::default());

        // Ensure the correct number of moves are generated
        assert_eq!(moves.len(), 12);
    }

    #[test]
    fn test_piece_move_generation() {
        init();
        let m = MoveGen::default();
        let white_pawn_moves = MoveGen::gen_pawn_moves(&m, Side::White);
        let black_pawn_moves = MoveGen::gen_pawn_moves(&m, Side::Black);
        let knight_moves = MoveGen::gen_knight_moves(&m, Side::White);
        let rook_moves = MoveGen::gen_rook_moves(&m, Side::White);
        let bishop_moves = MoveGen::gen_bishop_moves(&m, Side::White);
        let queen_moves = MoveGen::gen_queen_moves(&m, Side::White);
        let king_moves = MoveGen::gen_king_moves(&m, Side::White);

        assert!(!white_pawn_moves.is_empty());
        assert!(!black_pawn_moves.is_empty());
        assert!(!knight_moves.is_empty());
        assert!(!rook_moves.is_empty());
        assert!(!bishop_moves.is_empty());
        assert!(!queen_moves.is_empty());
        assert!(!king_moves.is_empty());
    }
    #[test]
    fn test_square_not_attacked() {
        init();
        // Initial position, e3 is not attacked by either side
        let board = Board::new();
        let moves = MoveGen::new(Piece::Pawn, Side::White, &board);
        println!("{board}");

        assert!(!moves.is_square_attacked(Square::from_str("e5").unwrap().index(), Side::White));
        assert!(!moves.is_square_attacked(Square::from_str("e4").unwrap().index(), Side::Black));
    }

    #[test]
    fn test_pawn_attacks() {
        init();
        // Create a position where pawns attack squares
        let board = Board::from_fen("8/8/8/8/3p4/8/2P5/8 w - - 0 1");
        let moves = MoveGen::new(Piece::Pawn, Side::White, &board);

        println!("{board}");

        // c2 white pawn attacks d3 and b3
        assert!(moves.is_square_attacked(Square::from_str("d3").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("b3").unwrap().index(), Side::White));

        // d4 black pawn attacks e3 and c3
        assert!(moves.is_square_attacked(Square::from_str("e3").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("c3").unwrap().index(), Side::Black));
    }

    #[test]
    fn test_knight_attacks() {
        init();
        // Create a position with knights
        let board = Board::from_fen("8/8/8/3n4/8/8/3N4/8 w - - 0 1");
        let moves = MoveGen::new(Piece::Knight, Side::White, &board);
        println!("{board}");

        // White knight at d2 attacks various squares
        assert!(moves.is_square_attacked(Square::from_str("b3").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("f3").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("e4").unwrap().index(), Side::White));

        // Black knight at d5 attacks various squares
        assert!(moves.is_square_attacked(Square::from_str("f4").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("f6").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("e7").unwrap().index(), Side::Black));
    }

    #[test]
    fn test_bishop_attacks() {
        init();
        // Create a position with bishops
        let board = Board::from_fen("8/8/8/3b4/8/8/3B4/8 w - - 0 1");
        let moves = MoveGen::new(Piece::Bishop, Side::White, &board);
        println!("{board}");

        // White bishop at d2 attacks various squares
        assert!(moves.is_square_attacked(Square::from_str("c3").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("e3").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("c1").unwrap().index(), Side::White));

        // Black bishop at d5 attacks various squares
        assert!(moves.is_square_attacked(Square::from_str("c6").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("e4").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("e6").unwrap().index(), Side::Black));
    }

    #[test]
    fn test_rook_attacks() {
        init();
        // Create a position with rooks
        let board = Board::from_fen("8/8/8/3r4/8/8/3R4/8 w - - 0 1");
        let moves = MoveGen::new(Piece::Rook, Side::White, &board);
        println!("{board}");

        // White rook at d2 attacks various squares
        assert!(moves.is_square_attacked(Square::from_str("d1").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("d3").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("a2").unwrap().index(), Side::White));

        // Black rook at d5 attacks various squares
        assert!(moves.is_square_attacked(Square::from_str("d3").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("d6").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("h5").unwrap().index(), Side::Black));
    }

    #[test]
    fn test_queen_attacks() {
        init();
        // Create a position with queens
        let board = Board::from_fen("8/8/8/3q4/8/8/3Q4/8 w - - 0 1");
        let moves = MoveGen::new(Piece::Queen, Side::White, &board);
        println!("{board}");

        // White queen at d2 attacks various squares (bishop-like)
        assert!(moves.is_square_attacked(Square::from_str("c3").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("e3").unwrap().index(), Side::White));

        // White queen at d2 attacks various squares (rook-like)
        assert!(moves.is_square_attacked(Square::from_str("d1").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("d3").unwrap().index(), Side::White));

        // Black queen at d5 attacks various squares (bishop-like)
        assert!(moves.is_square_attacked(Square::from_str("c6").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("e4").unwrap().index(), Side::Black));

        // Black queen at d5 attacks various squares (rook-like)
        assert!(moves.is_square_attacked(Square::from_str("d3").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("h5").unwrap().index(), Side::Black));
    }

    #[test]
    fn test_king_attacks() {
        init();
        // Create a position with kings
        let board = Board::from_fen("8/8/8/3k4/8/8/3K4/8 w - - 0 1");
        let moves = MoveGen::new(Piece::King, Side::White, &board);
        println!("{board}");

        // White king at d2 attacks adjacent squares
        assert!(moves.is_square_attacked(Square::from_str("e2").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("c2").unwrap().index(), Side::White));
        assert!(moves.is_square_attacked(Square::from_str("d3").unwrap().index(), Side::White));

        // Black king at d5 attacks adjacent squares
        assert!(moves.is_square_attacked(Square::from_str("c5").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("e5").unwrap().index(), Side::Black));
        assert!(moves.is_square_attacked(Square::from_str("d6").unwrap().index(), Side::Black));
    }

    #[test]
    fn test_multiple_attackers() {
        init();
        // Position with multiple attackers on the same square
        let board = Board::from_fen("8/8/8/2bn4/8/4N3/8/8 w - - 0 1");
        let moves = MoveGen::new(Piece::Knight, Side::White, &board);

        // e3 square is attacked by both black bishop at c5 and black knight at d5
        assert!(moves.is_square_attacked(Square::from_str("e3").unwrap().index(), Side::Black));
    }

    #[test]
    fn test_blocked_attacks() {
        init();
        // Create a position where attacks are blocked
        let board = Board::from_fen("8/8/8/3r4/3P4/8/3R4/8 w - - 0 1");
        let moves = MoveGen::new(Piece::Rook, Side::White, &board);

        // White rook at d2 attacks d3 but not d5 (blocked by d4 pawn)
        assert!(moves.is_square_attacked(Square::from_str("d3").unwrap().index(), Side::White));
        assert!(!moves.is_square_attacked(Square::from_str("d5").unwrap().index(), Side::White));

        // Black rook at d5 attacks d4 but not d2 (blocked by d4 pawn)
        assert!(moves.is_square_attacked(Square::from_str("d4").unwrap().index(), Side::Black));
        assert!(!moves.is_square_attacked(Square::from_str("d3").unwrap().index(), Side::Black));
    }

    #[cfg(test)]
    mod en_passant_tests {
        use super::*;
        use crate::init;
        use std::str::FromStr;

        #[test]
        fn test_white_en_passant_capture_right() {
            init();
            // Set up position where white can en passant capture to the right
            let mut board = Board::from_fen("8/8/8/8/4p3/8/3P4/8 w - - 0 1");

            // Move white pawn from d2 to d4 (double push)
            let from_d2 = Square::from_str("d2").unwrap();
            let to_d4 = Square::from_str("d4").unwrap();

            board.try_move(from_d2, to_d4).unwrap();

            // Verify en passant square is set correctly
            assert_eq!(
                board.enpassant_square,
                Some(Square::from_str("d3").unwrap())
            );

            // Move black pawn to capture en passant
            board.stm = Side::Black; // Make sure it's black's turn

            let from_e4 = Square::from_str("e4").unwrap();
            let to_d3 = Square::from_str("d3").unwrap(); // en passant square

            assert!(board.is_move_legal(from_e4, to_d3));

            board.try_move(from_e4, to_d3).unwrap();

            // Verify the white pawn was captured
            assert!(
                !board
                    .positions
                    .get_piece_bb(&Side::White, &Piece::Pawn)
                    .contains_square(to_d4.index())
            ); // d4 should be empty

            // Verify black pawn moved to d3
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::Black, &Piece::Pawn)
                    .contains_square(to_d3.index())
            ); // Black pawn at d3

            // En passant square should be reset
            assert_eq!(board.enpassant_square, None);
        }

        #[test]
        fn test_white_en_passant_capture_left() {
            init();
            // Set up position where white can en passant capture to the left
            let mut board = Board::from_fen("8/8/8/8/4p3/8/5P2/8 w - - 0 1");

            // Move white pawn from f2 to f4 (double push)
            let from_f2 = Square::from_str("f2").unwrap();
            let to_f4 = Square::from_str("f4").unwrap();

            board.try_move(from_f2, to_f4).unwrap();

            // Verify en passant square is set correctly
            assert_eq!(
                board.enpassant_square,
                Some(Square::from_str("f3").unwrap())
            );

            // Move black pawn to capture en passant
            board.stm = Side::Black; // Make sure it's black's turn

            let from_e4 = Square::from_str("e4").unwrap();
            let to_f3 = Square::from_str("f3").unwrap(); // en passant square

            assert!(board.is_move_legal(from_e4, to_f3));

            board.try_move(from_e4, to_f3).unwrap();

            // Verify the white pawn was captured
            assert!(
                !board
                    .positions
                    .get_piece_bb(&Side::White, &Piece::Pawn)
                    .contains_square(to_f4.index())
            ); // f4 should be empty

            // Verify black pawn moved to f3
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::Black, &Piece::Pawn)
                    .contains_square(to_f3.index())
            ); // Black pawn at f3

            // En passant square should be reset
            assert_eq!(board.enpassant_square, None);
        }

        #[test]
        fn test_black_en_passant_capture_right() {
            init();
            // Set up position where black can en passant capture to the right
            let mut board = Board::from_fen("8/3p4/8/4P3/8/8/8/8 b - - 0 1");
            println!("{board}");

            // Move black pawn from d7 to d5 (double push)
            let from_d7 = Square::from_str("d7").unwrap();
            let to_d5 = Square::from_str("d5").unwrap();

            board.try_move(from_d7, to_d5).unwrap();
            println!("{board}");

            // Verify en passant square is set correctly
            assert_eq!(
                board.enpassant_square,
                Some(Square::from_str("d6").unwrap())
            );

            // Move white pawn to capture en passant
            board.stm = Side::White; // Make sure it's white's turn

            let from_e5 = Square::from_str("e5").unwrap();
            let to_d6 = Square::from_str("d6").unwrap(); // en passant square

            assert!(board.is_move_legal(from_e5, to_d6));

            board.try_move(from_e5, to_d6).unwrap();
            println!("{board}");

            // Verify the black pawn was captured
            assert!(
                !board
                    .positions
                    .get_piece_bb(&Side::Black, &Piece::Pawn)
                    .contains_square(to_d5.index())
            ); // d5 should be empty

            // Verify white pawn moved to d6
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::White, &Piece::Pawn)
                    .contains_square(to_d6.index())
            ); // White pawn at d6

            // En passant square should be reset
            assert_eq!(board.enpassant_square, None);
        }

        #[test]
        fn test_black_en_passant_capture_left() {
            init();
            // Set up position where black can en passant capture to the left
            let mut board = Board::from_fen("8/5p2/8/8/8/8/8/8 b - - 0 1");
            println!("{board}");

            // Move black pawn from f7 to f5 (double push)
            let from_f7 = Square::from_str("f7").unwrap();
            let to_f5 = Square::from_str("f5").unwrap();

            board.try_move(from_f7, to_f5).unwrap();

            // Verify en passant square is set correctly
            assert_eq!(
                board.enpassant_square,
                Some(Square::from_str("f6").unwrap())
            );

            // Move white pawn to capture en passant
            board.stm = Side::White; // Make sure it's white's turn

            // Place a white pawn at e5
            board
                .positions
                .set(
                    &Side::White,
                    &Piece::Pawn,
                    Square::from_str("e5").unwrap().index(),
                )
                .unwrap();

            let from_e5 = Square::from_str("e5").unwrap();
            let to_f6 = Square::from_str("f6").unwrap(); // en passant square

            assert!(board.is_move_legal(from_e5, to_f6));

            board.try_move(from_e5, to_f6).unwrap();

            // Verify the black pawn was captured
            assert!(
                !board
                    .positions
                    .get_piece_bb(&Side::Black, &Piece::Pawn)
                    .contains_square(to_f5.index())
            ); // f5 should be empty

            // Verify white pawn moved to f6
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::White, &Piece::Pawn)
                    .contains_square(to_f6.index())
            ); // White pawn at f6

            // En passant square should be reset
            assert_eq!(board.enpassant_square, None);
        }

        #[test]
        fn test_en_passant_opportunity_expires() {
            init();
            // Set up position where black can en passant capture
            let mut board = Board::from_fen("8/8/8/8/8/8/2P5/8 w - - 0 1");

            // Move white pawn from c2 to c4 (double push)
            let from_c2 = Square::from_str("c2").unwrap();
            let to_c4 = Square::from_str("c4").unwrap();

            board.try_move(from_c2, to_c4).unwrap();

            // Verify en passant square is set correctly
            assert_eq!(
                board.enpassant_square,
                Some(Square::from_str("c3").unwrap())
            );

            // Place a black pawn at d4
            board
                .positions
                .set(
                    &Side::Black,
                    &Piece::Pawn,
                    Square::from_str("d4").unwrap().index(),
                )
                .unwrap();

            // Make a different move with black instead of capturing en passant
            board.stm = Side::Black;
            let other_from = Square::from_str("d4").unwrap();
            let other_to = Square::from_str("d3").unwrap();

            board.try_move(other_from, other_to).unwrap();

            // En passant opportunity should expire
            assert_eq!(board.enpassant_square, None);

            // Make a move with white
            board.stm = Side::White;
            let from = Square::from_str("c4").unwrap();
            let to = Square::from_str("c5").unwrap();

            board.try_move(from, to).unwrap();

            // Now black shouldn't be able to capture en passant since opportunity expired
            board.stm = Side::Black;

            // Place a black pawn at b4
            board
                .positions
                .set(
                    &Side::Black,
                    &Piece::Pawn,
                    Square::from_str("b4").unwrap().index(),
                )
                .unwrap();

            let en_passant_from = Square::from_str("b4").unwrap();
            let en_passant_to = Square::from_str("c3").unwrap(); // former en passant square

            assert!(!board.is_move_legal(en_passant_from, en_passant_to));
        }

        #[test]
        fn test_en_passant_doesnt_leave_king_in_check() {
            init();
            // Set up position where en passant would leave king in check from a rook
            let board = Board::from_fen("4k2R/8/8/8/2pP4/8/8/4K2r b - d3 0 1");
            println!("{board}");

            // Try to capture en passant
            let from_c4 = Square::from_str("c4").unwrap();
            let to_d3 = Square::from_str("d3").unwrap(); // en passant square

            // This move should be illegal because it would leave the black king in check
            assert!(!board.is_move_legal(from_c4, to_d3));
        }
    }

    #[cfg(test)]
    mod castling_tests {

        use super::*;
        use std::str::FromStr;

        #[test]
        fn test_white_kingside_castling() {
            init();
            // Position where white can castle kingside
            let mut board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");

            // King from e1 to g1
            let from = Square::from_str("e1").unwrap();
            let to = Square::from_str("g1").unwrap();

            // Ensure the move is legal
            assert!(board.is_move_legal(from, to));

            // Make the move
            board.try_move(from, to).unwrap();

            // Verify king and rook positions after castling
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::White, &Piece::King)
                    .contains_square(Square::from_str("g1").unwrap().index())
            ); // King at g1
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::White, &Piece::Rook)
                    .contains_square(Square::from_str("f1").unwrap().index())
            ); // Rook at f1

            // Verify castling rights are updated
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::WHITE_00))
            );
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::WHITE_000))
            );
        }

        #[test]
        fn test_white_queenside_castling() {
            init();
            // Position where white can castle queenside
            let mut board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");

            // King from e1 to c1
            let from = Square::from_str("e1").unwrap();
            let to = Square::from_str("c1").unwrap();
            println!("{from}, {to}");

            println!("{board}");

            // Ensure the move is legal
            assert!(board.is_move_legal(from, to));

            // Make the move
            board
                .try_move(from, to)
                .inspect_err(|e| println!("{}", e))
                .expect("yo");

            println!("after move {board}");
            // Verify king and rook positions after castling
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::White, &Piece::King)
                    .contains_square(Square::from_str("c1").unwrap().index())
            ); // King at c1
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::White, &Piece::Rook)
                    .contains_square(Square::from_str("d1").unwrap().index())
            ); // Rook at d1

            // Verify castling rights are updated
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::WHITE_00))
            );
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::WHITE_000))
            );
        }

        #[test]
        fn test_black_kingside_castling() {
            init();
            // Position where black can castle kingside
            let mut board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1");
            board.stm = Side::Black;

            // King from e8 to g8
            let from = Square::from_str("e8").unwrap();
            let to = Square::from_str("g8").unwrap();

            // Ensure the move is legal
            assert!(board.is_move_legal(from, to));

            // Make the move
            board
                .try_move(from, to)
                .inspect_err(|e| println!("{}", e))
                .expect("yo");

            // Verify king and rook positions after castling
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::Black, &Piece::King)
                    .contains_square(Square::from_str("g8").unwrap().index())
            ); // King at g8
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::Black, &Piece::Rook)
                    .contains_square(Square::from_str("f8").unwrap().index())
            ); // Rook at f8

            // Verify castling rights are updated
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::BLACK_00))
            );
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::BLACK_000))
            );
        }

        #[test]
        fn test_black_queenside_castling() {
            init();
            // Position where black can castle queenside
            let mut board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1");

            // King from e8 to c8
            let from = Square::from_str("e8").unwrap();
            let to = Square::from_str("c8").unwrap();

            // Ensure the move is legal
            assert!(board.is_move_legal(from, to));

            // Make the move
            board.try_move(from, to).unwrap();

            // Verify king and rook positions after castling
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::Black, &Piece::King)
                    .contains_square(Square::from_str("c8").unwrap().index())
            ); // King at c8
            assert!(
                board
                    .positions
                    .get_piece_bb(&Side::Black, &Piece::Rook)
                    .contains_square(Square::from_str("d8").unwrap().index())
            ); // Rook at d8

            // Verify castling rights are updated
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::BLACK_00))
            );
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::BLACK_000))
            );
        }

        #[test]
        fn test_castling_through_check() {
            init();
            // Position where white cannot castle through check
            let board = Board::from_fen("r3k2r/pppppppp/8/8/8/4n3/PPPPPPPP/R3K2R w KQkq - 0 1");
            println!("{board}");

            // King from e1 to g1 (would castle through check on f1)
            let from = Square::from_str("e1").unwrap();
            let to = Square::from_str("g1").unwrap();

            // Ensure the move is illegal
            assert!(!board.is_move_legal(from, to));
        }

        #[test]
        fn test_castling_out_of_check() {
            init();
            // Position where white is in check and cannot castle
            let mut board = Board::from_fen("r3k2r/ppp1pppp/8/8/8/8/PPPPqPPP/R3K2R w KQkq - 0 1");
            board.stm = Side::Black;

            // King from e1 to g1
            let from = Square::from_str("e1").unwrap();
            let to = Square::from_str("g1").unwrap();
            let res = board.is_move_legal(from, to);

            // Ensure the move is illegal
            assert!(!res);
        }

        #[test]
        fn test_castling_into_check() {
            init();
            // Position where white cannot castle into check
            let board = Board::from_fen("r3k2r/pppppppp/8/8/8/5n2/PPPPPPPP/R3K2R w KQkq - 0 1");

            // King from e1 to g1 (would end up in check on g1)
            let from = Square::from_str("e1").unwrap();
            let to = Square::from_str("g1").unwrap();

            let res = board.is_move_legal(from, to);

            // Ensure the move is illegal
            assert!(!res);
        }

        #[test]
        fn test_castling_with_blocked_squares() {
            init();
            // Position where castling is blocked by pieces
            let board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R1N1K2R w KQkq - 0 1");
            println!("{board}");

            // King from e1 to c1 (blocked by knight on c1)
            let from = Square::from_str("e1").unwrap();
            let to = Square::from_str("c1").unwrap();

            // Ensure the move is illegal
            assert!(!board.is_move_legal(from, to));
        }

        #[test]
        fn test_castling_rights_after_king_move() {
            init();
            // Position with all castling rights
            let mut board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");

            // Move king without castling
            let from = Square::from_str("e1").unwrap();
            let to = Square::from_str("f1").unwrap();

            board.try_move(from, to).unwrap();

            // Verify castling rights are removed for white
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::WHITE_00))
            );
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::WHITE_000))
            );

            // Black castling rights should still be intact
            assert!(
                board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::BLACK_00))
            );
            assert!(
                board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::BLACK_000))
            );
        }

        #[test]
        fn test_castling_rights_after_rook_move() {
            init();
            // Position with all castling rights
            let mut board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");

            // Move kingside rook
            let from = Square::from_str("h1").unwrap(); // h1
            let to = Square::from_str("g1").unwrap(); // h2

            board.try_move(from, to).unwrap();

            // Verify kingside castling right is removed for white
            assert!(
                !board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::WHITE_00))
            );

            // Queenside castling right for white should still be intact
            assert!(
                board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::WHITE_000))
            );
        }
    }
}
