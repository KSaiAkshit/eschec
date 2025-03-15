use std::{collections::HashMap, fmt::Display};

use moves::Moves;

use self::components::{BoardState, CastlingRights, Piece, Side, Square};

pub mod components;
mod fen;
pub mod moves;

/// Completely encapsulate the game
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
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

impl Default for Board {
    fn default() -> Self {
        Self {
            positions: BoardState::default(),
            stm: Side::default(),
            castling_rights: CastlingRights::empty(),
            enpassant_square: None,
            halfmove_clock: 0,
            fullmove_counter: 1,
            material: [0, 0],
        }
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Define Unicode chess pieces
        let piece_chars = [
            ['♙', '♗', '♘', '♖', '♕', '♔'], // White pieces
            ['♟', '♝', '♞', '♜', '♛', '♚'], // Black pieces
        ];

        // Create top border with file labels
        writeln!(f, "  +---+---+---+---+---+---+---+---+")?;

        // Iterate through ranks (from top to bottom)
        for rank in (0..8).rev() {
            // Write rank number
            write!(f, "{} |", rank + 1)?;

            // Iterate through files (from left to right)
            for file in 0..8 {
                let square_idx = rank * 8 + file;

                // Check if there's a piece at this square
                let mut piece_found = false;

                for (piece_type, side) in Piece::all() {
                    let side_idx = side.index();
                    let piece_idx = piece_type.index();

                    if self.positions.all_pieces[side_idx][piece_idx].contains_square(square_idx) {
                        // Draw the piece using Unicode chess piece
                        write!(f, " {} |", piece_chars[side_idx][piece_idx])?;
                        piece_found = true;
                        break;
                    }
                }

                // If no piece found, draw empty square
                if !piece_found {
                    // Use different background for alternating squares
                    let is_dark = (rank + file) % 2 == 1;
                    if is_dark {
                        write!(f, " · |")?;
                    } else {
                        write!(f, "   |")?;
                    }
                }
            }

            // End of rank
            writeln!(f)?;
            writeln!(f, "  +---+---+---+---+---+---+---+---+")?;
        }

        // File letters at the bottom
        writeln!(f, "    A   B   C   D   E   F   G   H  ")?;

        // Additional game information
        writeln!(f, "\nSide to move: {}", self.stm)?;
        writeln!(f, "Castling rights: {}", self.castling_rights)?;

        if let Some(ep) = self.enpassant_square {
            writeln!(f, "En passant square: {}", ep)?;
        } else {
            writeln!(f, "En passant square: -")?;
        }

        writeln!(f, "Halfmove clock: {}", self.halfmove_clock)?;
        writeln!(f, "Fullmove counter: {}", self.fullmove_counter)?;
        writeln!(
            f,
            "Material balance: W:{} B:{}",
            self.material[Side::White.index()],
            self.material[Side::Black.index()]
        )?;

        Ok(())
    }
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
                let moves = Moves::new(piece, self.stm);
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
        let moves = Moves::new(piece, self.stm);
        let legal_squares = moves.attack_bb[from.index()];

        // 4. Check if the 'to' square is a legal square for the piece
        if !legal_squares.contains_square(to.index()) {
            return Ok(false);
        }

        // 5. Check if the move puts own king in check
        let mut board = *self;
        board.try_move(from, to);
        Ok(!board.is_in_check(self.stm))
    }

    fn try_move(&mut self, from: Square, to: Square) {
        if let Some(piece) = self.get_piece_at(from) {
            self.positions
                .update_piece_position(&piece, &self.stm, from, to);

            self.handle_special_rules(from, to);
            self.calculate_material();
            self.halfmove_clock += 1;
            if self.stm == Side::White {
                self.fullmove_counter += 1;
            }
        }
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
            let moves = Moves::new(piece, self.stm);
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
                    file += c.to_digit(10).unwrap() as usize;
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

    // fn is_insufficient_material(&self) -> bool {
    //     let white_pieces = self.positions.all_sides[Side::White.index()];
    //     let black_pieces = self.positions.all_sides[Side::Black.index()];

    //     let mut white_counts = [0; 6];
    //     let mut black_counts = [0; 6];

    //     for piece in Piece::PIECES.iter() {
    //         white_counts[piece.index()] = self.positions.all_pieces[Side::White.index()]
    //             [piece.index()]
    //         .get_set_bits()
    //         .len();
    //         black_counts[piece.index()] = self.positions.all_pieces[Side::Black.index()]
    //             [piece.index()]
    //         .get_set_bits()
    //         .len();
    //     }

    //     if white_pieces.0.count_ones() == 1 && black_pieces.0.count_ones() == 1 {
    //         return true;
    //     }
    //     // King and Bishop vs King
    //     if (white_counts[Piece::King.index()] == 1
    //         && white_counts[Piece::Bishop.index()] == 1
    //         && white_pieces.0.count_ones() == 2
    //         && black_counts[Piece::King.index()] == 1
    //         && black_pieces.0.count_ones() == 1)
    //         || (black_counts[Piece::King.index()] == 1
    //             && black_counts[Piece::Bishop.index()] == 1
    //             && black_pieces.0.count_ones() == 2
    //             && white_counts[Piece::King.index()] == 1
    //             && white_pieces.0.count_ones() == 1)
    //     {
    //         return true;
    //     }

    //     // King and Knight vs King
    //     if (white_counts[Piece::King.index()] == 1
    //         && white_counts[Piece::Knight.index()] == 1
    //         && white_pieces.0.count_ones() == 2
    //         && black_counts[Piece::King.index()] == 1
    //         && black_pieces.0.count_ones() == 1)
    //         || (black_counts[Piece::King.index()] == 1
    //             && black_counts[Piece::Knight.index()] == 1
    //             && black_pieces.0.count_ones() == 2
    //             && white_counts[Piece::King.index()] == 1
    //             && white_pieces.0.count_ones() == 1)
    //     {
    //         return true;
    //     }

    //     // King and Bishop vs King and Bishop (same colored squares)
    //     if white_counts[Piece::King.index()] == 1
    //         && white_counts[Piece::Bishop.index()] == 1
    //         && black_counts[Piece::King.index()] == 1
    //         && black_counts[Piece::Bishop.index()] == 1
    //         && self.same_colored_bishops()
    //     {
    //         return true;
    //     }

    //     false
    // }

    fn is_insufficient_material(&self) -> bool {
        let white_pieces = self.positions.all_sides[Side::White.index()];
        let black_pieces = self.positions.all_sides[Side::Black.index()];

        // Arrays to store the counts of each piece type
        let mut white_counts = [0; 6];
        let mut black_counts = [0; 6];

        // Count the pieces for both sides
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

        // If both sides have only their kings, it's insufficient material
        if white_pieces.0.count_ones() == 1 && black_pieces.0.count_ones() == 1 {
            return true;
        }

        // King and Bishop vs King (White or Black can have King and Bishop)
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

        // King and Knight vs King (White or Black can have King and Knight)
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

        // Additional check for opposite colored bishops (same as insufficient material)
        if white_counts[Piece::King.index()] == 1
            && white_counts[Piece::Bishop.index()] == 1
            && black_counts[Piece::King.index()] == 1
            && black_counts[Piece::Bishop.index()] == 1
            && !self.same_colored_bishops()
        {
            return true;
        }

        // If none of the above conditions are met, it's not insufficient material
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

            let a = (white_file + white_rank) % 2;
            let b = (black_file + black_rank) % 2;
            a == b
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
        let mut board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKBNR w KQkq - 0 1");
        board.calculate_material();
        // Standard position with a missing f2 pawn on white's side

        assert_eq!(board.material[Side::White.index()], 39); // 40 - 1 = 39
        assert_eq!(board.material[Side::Black.index()], 40);
    }

    #[test]
    fn test_king_vs_king() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_king_and_bishop_vs_king() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KB2 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_king_and_knight_vs_king() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KN2 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_kings_and_same_colored_bishops() {
        // Bishops on the same color squares (both on light squares)
        let board = Board::from_fen("2b1k3/8/8/8/8/8/8/2B1K3 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_kings_and_different_colored_bishops() {
        // Bishops on different color squares (one on light, one on dark)
        let board = Board::from_fen("1b2k3/8/8/8/8/8/8/2B1K3 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_sufficient_material() {
        let board = Board::from_fen("4k3/8/8/8/8/8/4P3/4KB2 w - - 0 1");

        assert!(!board.is_insufficient_material());
    }

    #[test]
    fn test_two_knights_sufficient_material() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KNN1 w - - 0 1");

        assert!(!board.is_insufficient_material());
    }

    #[test]
    fn test_two_bishops_sufficient_material() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/3BKB2 w - - 0 1");

        assert!(!board.is_insufficient_material());
    }
}
