use std::collections::HashMap;

use moves::Moves;

use self::components::{BoardState, CastlingRights, Piece, Side, Square};

pub mod components;
mod fen;
pub mod moves;

/// Completely encapsulate the game
#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Board {
    // Snapshot of current board
    pub positions: BoardState,
    /// Side to move, 0 - white, 1 - black
    pub stm: Side,
    /// Castling rights for both sides, KQ - White king&queen side, kq - Black king&queen side, '-' no right
    pub castling_rights: CastlingRights,
    /// En passant target square
    pub enpassant_square: Option<Square>,
    /// Specifies a number of half-moves with respect to the 50 move draw rule. It is reset(to 0) after a capture
    /// or a pawn move and incremented otherwise.
    pub halfmove_clock: u8,
    ///  The number of the full moves in a game. It starts at 1, and is incremented after each Black's move.
    pub fullmove_counter: u8,
    /// Material left for each side [White, Black]
    pub material: [u64; 2],
}

impl Board {
    /// Use to initialize a default board
    pub fn new() -> Self {
        const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
        // const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let mut board = Self {
            positions: BoardState::default(),
            stm: Side::default(),
            castling_rights: CastlingRights::default(),
            enpassant_square: Option::default(),
            halfmove_clock: u8::default(),
            fullmove_counter: u8::default(),
            material: [u64::default(); 2],
        };
        match board.place_pieces(START_FEN) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error initializing board: {}", e);
            }
        }
        board
    }
    /// Use this to construct a board from fen
    pub fn from_fen(fen: &str) -> Self {
        let parsed = fen::parse_fen(fen);
        match parsed {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Got error while parsing given fen: {}", e);
                panic!("very bad fen")
            }
        }
    }

    pub fn generate_legal_moves(&self) -> anyhow::Result<Vec<(Square, Square)>> {
        // NOTE: Is this correct as well?
        let mut legal_moves = Vec::new();
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            if let Some(piece) = self.get_piece_at(square) {
                let moves = Moves::new(piece);
                moves.attack_bb.into_iter().for_each(|attack_bb| {
                    let targets = attack_bb.get_set_bits();
                    targets.into_iter().for_each(|target_index| {
                        let target_square = Square::new(target_index).expect("Get a valid index");
                        if self
                            .is_move_legal(square, target_square)
                            .expect("Should be able to check if it is a legal move")
                        {
                            legal_moves.push((square, target_square));
                        }
                    });
                });
            }
        });
        Ok(legal_moves)
    }

    pub fn make_move(&mut self, from: Square, to: Square) -> anyhow::Result<()> {
        if !self.is_move_legal(from, to)? {
            anyhow::bail!("Illegal move from {} to {}", from, to);
        }
        if let Some(piece) = self.get_piece_at(from) {
            self.positions
                .update_piece_position(&piece, &self.stm, from, to);

            self.handle_special_rules(from, to);
            self.calculate_material();
            self.stm = self.stm.flip();
            self.halfmove_clock += 1;
            if self.stm == Side::White {
                self.fullmove_counter += 1;
            }
            Ok(())
        } else {
            anyhow::bail!("No piece at from Square");
        }
    }

    pub fn is_move_legal(&self, from: Square, to: Square) -> anyhow::Result<bool> {
        // NOTE: Is this correct? seems like something is missing
        // 1. Check if there is a piece at the 'from' square
        let piece = match self.get_piece_at(from) {
            Some(p) => p,
            None => return Ok(false),
        };

        // 2. Check if the piece belongs to the current side to move
        if !self.positions.all_sides[self.stm.index()].contains_square(from.index()) {
            return Ok(false);
        }

        // 3. Generate legal moves for the piece
        let moves = Moves::new(piece);
        let legal_squares = moves.attack_bb[from.index()];

        // 4. Check if the 'to' square is a legal square for the piece
        if !legal_squares.contains_square(to.index()) {
            return Ok(false);
        }

        // 5. Check if the move puts own king in check
        let mut board = *self;
        board.make_move(from, to).unwrap();
        Ok(!board.is_in_check(self.stm))
    }

    pub fn is_in_check(&self, side: Side) -> bool {
        // 1. Find king's position
        let king_bb = self.positions.all_pieces[side.index()][Piece::King.index()];
        let king_square =
            Square::new(king_bb.get_set_bits()[0]).expect("Should be able to find king");

        // 2. Generate oppponent's attacks
        let opponent = side.flip();
        for piece in Piece::colored_pieces(opponent) {
            let piece_bb = self.positions.all_pieces[opponent.index()][piece.index()];
            let moves = Moves::new(piece);
            // If any opponent piece can attack king's square, king is in check
            if piece_bb
                .get_set_bits()
                .iter()
                .any(|&from| moves.attack_bb[from].contains_square(king_square.index()))
            {
                return true;
            }
        }

        false
    }

    pub fn is_checkmate(&self, side: Side) -> bool {
        self.is_in_check(side)
            && !self
                .generate_legal_moves()
                .expect("Should be able to gen legal moves")
                .is_empty()
    }

    pub fn is_stalemate(&self, side: Side) -> bool {
        !self.is_in_check(side)
            && self
                .generate_legal_moves()
                .expect("Should be able to gen legal moves")
                .is_empty()
    }

    pub fn is_draw(&self) -> bool {
        self.is_stalemate(self.stm) || self.halfmove_clock >= 100 || self.is_insufficient_material()
    }

    // NOTE: Older Implementation without support for full length FEN strings
    fn place_pieces(&mut self, fen: &str) -> anyhow::Result<()> {
        if fen.contains(' ') {
            return Err(anyhow::Error::msg("Not supported for now"));
        }
        let lookup_table: HashMap<char, (usize, usize)> = [
            ('P', (Piece::Pawn as usize, Side::White as usize)),
            ('p', (Piece::Pawn as usize, Side::Black as usize)),
            ('B', (Piece::Bishop as usize, Side::White as usize)),
            ('b', (Piece::Bishop as usize, Side::Black as usize)),
            ('N', (Piece::Knight as usize, Side::White as usize)),
            ('n', (Piece::Knight as usize, Side::Black as usize)),
            ('R', (Piece::Rook as usize, Side::White as usize)),
            ('r', (Piece::Rook as usize, Side::Black as usize)),
            ('Q', (Piece::Queen as usize, Side::White as usize)),
            ('q', (Piece::Queen as usize, Side::Black as usize)),
            ('K', (Piece::King as usize, Side::White as usize)),
            ('k', (Piece::King as usize, Side::Black as usize)),
        ]
        .into_iter()
        .collect();
        // rank [7,0]
        let mut rank = 7;
        // file [0,7]
        let mut file = 0;
        // dbg!(rank, file);
        for c in fen.chars() {
            // dbg!("---------------------");
            // dbg!(c);
            match c {
                '1'..='8' => {
                    // dbg!(rank, file);
                    file += c.to_digit(10).unwrap() as usize - 1;
                    // dbg!(rank, file);
                }
                '/' => {
                    // dbg!(rank, file);
                    rank -= 1;
                    file = 0;
                    // dbg!(rank, file);
                }
                _ => {
                    if let Some((piece, side)) = lookup_table.get(&c) {
                        // dbg!(piece, side);
                        // dbg!(rank, file);
                        self.positions.all_pieces[*side][*piece].set(rank * 8 + file);
                        file += 1;
                    } else {
                        return Err(anyhow::Error::msg("Invalid Fen Character"));
                    }
                }
            }
        }
        // updte all_sides too
        self.positions.all_sides[0] = self.positions.all_pieces[0][0]
            | self.positions.all_pieces[0][1]
            | self.positions.all_pieces[0][2]
            | self.positions.all_pieces[0][3]
            | self.positions.all_pieces[0][4]
            | self.positions.all_pieces[0][5];

        self.positions.all_sides[1] = self.positions.all_pieces[1][0]
            | self.positions.all_pieces[1][1]
            | self.positions.all_pieces[1][2]
            | self.positions.all_pieces[1][3]
            | self.positions.all_pieces[1][4]
            | self.positions.all_pieces[1][5];
        Ok(())
    }

    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        let index = square.index();

        for (piece_type, side) in Piece::all() {
            let piece_bb = self.positions.all_pieces[side as usize][piece_type as usize];
            if piece_bb.contains_square(index) {
                return Some(piece_type);
            }
        }
        None
    }

    pub fn handle_special_rules(&mut self, from: Square, to: Square) {
        // Handle castling
        if let Some(Piece::King) = self.get_piece_at(from) {
            // Update castling rights
            match self.stm {
                Side::White => self
                    .castling_rights
                    .remove_right(CastlingRights::WHITE_CASTLING),
                Side::Black => self
                    .castling_rights
                    .remove_right(CastlingRights::BLACK_CASTLING),
            }
            // Move rook if castling
            // TODO: Implement castling logic
        }

        // Handle en passant
        if let Some(Piece::Pawn) = self.get_piece_at(from) {
            // Set en passant square if double pawn push
            // A double pawn push moves two squares forward from starting position
            let is_double_push = match self.stm {
                Side::White => from.index() / 8 == 1 && to.index() / 8 == 3,
                Side::Black => from.index() / 8 == 6 && to.index() / 8 == 4,
            };

            // If double push detected, set en passant square to the skipped square
            if is_double_push {
                let skipped_square = Square::new(match self.stm {
                    Side::White => from.index() + 8,
                    Side::Black => from.index() - 8,
                })
                .unwrap();
                self.enpassant_square = Some(skipped_square);
            }

            // If moving to en passant square, capture the enemy pawn
            if let Some(ep_square) = self.enpassant_square {
                if to == ep_square {
                    let captured_pawn_square = Square::new(match self.stm {
                        Side::White => ep_square.index() - 8,
                        Side::Black => ep_square.index() + 8,
                    })
                    .unwrap();
                    self.positions.all_pieces[self.stm.flip().index()][Piece::Pawn.index()]
                        .capture(captured_pawn_square.index());
                }
            }
        }
    }

    fn calculate_material(&mut self) {
        // Reset material counts
        self.material = [0; 2];

        // Calculate material for each side
        for side in [Side::White, Side::Black] {
            let side_index = side.index();
            for piece in Piece::colored_pieces(side) {
                let piece_bb = self.positions.all_pieces[side_index][piece.index()];
                let piece_count = piece_bb.0.count_ones() as u64;
                let piece_value: u64 = u8::from(piece) as u64;
                self.material[side_index] += piece_count * piece_value;
            }
        }
    }

    fn is_insufficient_material(&self) -> bool {
        let white_pieces = self.positions.all_sides[Side::White.index()];
        let black_pieces = self.positions.all_sides[Side::Black.index()];

        let mut white_counts = [0; 6];
        let mut black_counts = [0; 6];

        for piece in Piece::PIECES.iter() {
            white_counts[piece.index()] = self.positions.all_pieces[Side::White.index()]
                [piece.index()]
            .get_set_bits()
            .len();
            black_counts[piece.index()] = self.positions.all_pieces[Side::Black.index()]
                [piece.index()]
            .get_set_bits()
            .len();
        }

        if white_pieces.0.count_ones() == 1 && black_pieces.0.count_ones() == 1 {
            return true;
        }
        // King and Bishop vs King
        if (white_counts[Piece::King.index()] == 1
            && white_counts[Piece::Bishop.index()] == 1
            && white_pieces.0.count_ones() == 2
            && black_counts[Piece::King.index()] == 1
            && black_pieces.0.count_ones() == 1)
            || (black_counts[Piece::King.index()] == 1
                && black_counts[Piece::Bishop.index()] == 1
                && black_pieces.0.count_ones() == 2
                && white_counts[Piece::King.index()] == 1
                && white_pieces.0.count_ones() == 1)
        {
            return true;
        }

        // King and Knight vs King
        if (white_counts[Piece::King.index()] == 1
            && white_counts[Piece::Knight.index()] == 1
            && white_pieces.0.count_ones() == 2
            && black_counts[Piece::King.index()] == 1
            && black_pieces.0.count_ones() == 1)
            || (black_counts[Piece::King.index()] == 1
                && black_counts[Piece::Knight.index()] == 1
                && black_pieces.0.count_ones() == 2
                && white_counts[Piece::King.index()] == 1
                && white_pieces.0.count_ones() == 1)
        {
            return true;
        }

        // King and Bishop vs King and Bishop (same colored squares)
        if white_counts[Piece::King.index()] == 1
            && white_counts[Piece::Bishop.index()] == 1
            && black_counts[Piece::King.index()] == 1
            && black_counts[Piece::Bishop.index()] == 1
            && self.same_colored_bishops()
        {
            return true;
        }

        false
    }

    fn same_colored_bishops(&self) -> bool {
        let white_bishop = self.positions.all_pieces[Side::White.index()][Piece::Bishop.index()];
        let black_bishop = self.positions.all_pieces[Side::Black.index()][Piece::Bishop.index()];

        if let (Some(white_sq), Some(black_sq)) = (
            white_bishop.get_set_bits().first(),
            black_bishop.get_set_bits().first(),
        ) {
            // Bishops are on same colored squares if sum of their coordinates is even/odd same
            let (white_file, white_rank) = Square::new(*white_sq).unwrap().coords();
            let (black_file, black_rank) = Square::new(*black_sq).unwrap().coords();

            (white_file + white_rank) % 2 == (black_file + black_rank) % 2
        } else {
            false
        }
    }
}

#[cfg(test)]
mod material_tests {

    use super::*;

    #[test]
    fn test_initial_material_balance() {
        let mut board = Board::new();
        board.calculate_material();
        // Initial material for each side should be:
        // 8 pawns (8 * 1 = 8)
        // 2 knights (2 * 3 = 6)
        // 2 bishops (2 * 3 = 6)
        // 2 rooks (2 * 5 = 10)
        // 1 queen (1 * 9 = 9)
        // 1 king (1 * 1 = 1)
        // Total: 40
        assert_eq!(board.material[Side::White.index()], 40);
        assert_eq!(board.material[Side::Black.index()], 40);
    }

    #[test]
    fn test_material_after_capture() {
        let mut board = Board::new();

        // Simulate capturing a pawn
        board.positions.all_pieces[Side::White.index()][Piece::Pawn.index()].capture(8);
        board.calculate_material();

        assert_eq!(board.material[Side::White.index()], 39); // 40 - 1 = 39
        assert_eq!(board.material[Side::Black.index()], 40);
    }

    fn create_empty_board() -> Board {
        let mut board = Board::default();
        board.stm = Side::White;
        board.castling_rights = CastlingRights::empty();
        board.enpassant_square = None;
        board.halfmove_clock = 0;
        board.fullmove_counter = 1;
        board
    }

    #[test]
    fn test_king_vs_king() {
        let mut board = create_empty_board();

        // Place only kings
        board.positions.all_pieces[Side::White.index()][Piece::King.index()].set(4); // E1
        board.positions.all_pieces[Side::Black.index()][Piece::King.index()].set(60); // E8

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_king_and_bishop_vs_king() {
        let mut board = create_empty_board();

        // Place kings and one bishop
        board.positions.all_pieces[Side::White.index()][Piece::King.index()].set(4); // E1
        board.positions.all_pieces[Side::White.index()][Piece::Bishop.index()].set(5); // F1
        board.positions.all_pieces[Side::Black.index()][Piece::King.index()].set(60); // E8

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_king_and_knight_vs_king() {
        let mut board = create_empty_board();

        // Place kings and one knight
        board.positions.all_pieces[Side::White.index()][Piece::King.index()].set(4); // E1
        board.positions.all_pieces[Side::White.index()][Piece::Knight.index()].set(6); // G1
        board.positions.all_pieces[Side::Black.index()][Piece::King.index()].set(60); // E8

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_kings_and_same_colored_bishops() {
        let mut board = create_empty_board();

        // Place kings and bishops on same colored squares
        board.positions.all_pieces[Side::White.index()][Piece::King.index()].set(4); // E1
        board.positions.all_pieces[Side::White.index()][Piece::Bishop.index()].set(2); // C1 (light square)
        board.positions.all_pieces[Side::Black.index()][Piece::King.index()].set(60); // E8
        board.positions.all_pieces[Side::Black.index()][Piece::Bishop.index()].set(58); // C8 (light square)

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_sufficient_material() {
        let mut board = create_empty_board();

        // Place kings, bishop and pawn
        board.positions.all_pieces[Side::White.index()][Piece::King.index()].set(4); // E1
        board.positions.all_pieces[Side::White.index()][Piece::Bishop.index()].set(5); // F1
        board.positions.all_pieces[Side::White.index()][Piece::Pawn.index()].set(12); // E2
        board.positions.all_pieces[Side::Black.index()][Piece::King.index()].set(60); // E8

        assert!(!board.is_insufficient_material());
    }
}
