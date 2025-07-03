use crate::{
    evaluation::Evaluator,
    moves::{
        attack_data::calculate_attack_data,
        move_gen::generate_legal_moves,
        move_info::{Move, MoveInfo},
        precomputed::MOVE_TABLES,
    },
};
use miette::Context;
#[cfg(feature = "random")]
use rand::prelude::*;
use std::{collections::HashMap, fmt::Display};
use tracing::*;

use self::components::{BoardState, CastlingRights, Piece, Side, Square};

pub mod components;
pub mod fen;

/// Completely encapsulate the game
#[derive(Default, Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Copy)]
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

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
                    if self
                        .positions
                        .get_piece_bb(side, piece_type)
                        .contains_square(square_idx)
                    {
                        // Draw the piece using Unicode chess piece
                        write!(f, " {} |", piece_type.icon(side))?;
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
            writeln!(f, "En passant square: {ep}")?;
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
        let mut board = Self {
            positions: BoardState::default(),
            stm: Side::default(),
            castling_rights: CastlingRights::all(),
            enpassant_square: Option::default(),
            halfmove_clock: u8::default(),
            fullmove_counter: u8::default(),
            material: [u64::default(); 2],
        };
        match board.place_pieces(START_FEN) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error initializing board: {e}");
            }
        }
        board.calculate_material();
        board
    }
    /// Use this to construct a board from fen
    pub fn from_fen(fen: &str) -> Self {
        let parsed = fen::parse_fen(fen);
        let mut board = match parsed {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Got error while parsing given fen: {e}");
                panic!("very bad fen")
            }
        };
        board.calculate_material();
        board
    }

    pub fn to_fen(&self) -> miette::Result<String> {
        fen::to_fen(self)
    }

    pub fn generate_legal_moves(&self) -> Vec<Move> {
        let mut legal_moves = Vec::with_capacity(40);
        generate_legal_moves(self, &mut legal_moves);
        legal_moves
    }

    pub fn generate_legal_moves_for_search(&self) -> Vec<(Square, Square)> {
        let moves = self.generate_legal_moves();
        moves
            .into_iter()
            .map(|m| (m.from_sq(), m.to_sq()))
            .collect()
    }

    pub fn generate_piecewise_legal_moves(&self) -> HashMap<Piece, Vec<(Square, Square)>> {
        // 1. Generate all legal moves using the efficient new generator.
        let all_legal_moves = self.generate_legal_moves(); // This returns Vec<Move>

        let mut piecewise_moves: HashMap<Piece, Vec<(Square, Square)>> = HashMap::new();

        // 2. Iterate through the legal moves and group them by the piece that moved.
        for m in all_legal_moves {
            let from_sq = m.from_sq();
            let to_sq = m.to_sq();

            // We need to know which piece is at the 'from' square.
            // Since we know the move is legal, there will always be a piece there.
            if let Some(piece) = self.get_piece_at(from_sq) {
                piecewise_moves
                    .entry(piece)
                    .or_default() // Get the vec for this piece, or create a new one.
                    .push((from_sq, to_sq));
            }
        }

        piecewise_moves
    }

    pub fn try_move(&mut self, from: Square, to: Square) -> miette::Result<()> {
        let legal_moves = self.generate_legal_moves();
        let the_move = legal_moves
            .iter()
            .find(|m| m.from_sq() == from && m.to_sq() == to);

        if let Some(m) = the_move {
            self.make_move(*m)?;
            Ok(())
        } else {
            miette::bail!("Illegal move from {} to {}", from, to);
        }
    }

    pub fn try_move_with_info(&mut self, from: Square, to: Square) -> miette::Result<MoveInfo> {
        let legal_moves = self.generate_legal_moves();
        let the_move = legal_moves
            .iter()
            .find(|m| m.from_sq() == from && m.to_sq() == to);

        if let Some(m) = the_move {
            let move_data = self.make_move_with_info(*m)?;
            Ok(move_data)
        } else {
            miette::bail!("[try_move_with_info] Illegal move from {} to {}", from, to);
        }
    }

    pub fn unmake_move(&mut self, move_data: &MoveInfo) -> miette::Result<()> {
        self.stm = self.stm.flip();
        self.castling_rights = move_data.castle_rights;
        self.enpassant_square = move_data.enpassant_square;
        self.halfmove_clock = move_data.halfmove_clock;
        if self.stm == Side::Black {
            self.fullmove_counter -= 1;
        }

        let from = move_data.from;
        let to = move_data.to;
        let piece = move_data.piece_moved;

        // Move piece back
        self.positions
            .update_piece_position(piece, self.stm, to, from)?;

        // Handle promotions
        if let Some(_promo_piece) = move_data.promotion {
            // The piece at 'from' is now the promoted piece. We need to change it back to a pawn.
            self.positions.capture(self.stm, piece, from.index())?;
            self.positions.set(self.stm, Piece::Pawn, from.index())?;
        }

        // Put back captured piece
        if let Some(captured) = move_data.captured_piece {
            if move_data.is_en_passant {
                let captured_idx = match self.stm {
                    Side::White => to.index() - 8,
                    Side::Black => to.index() + 8,
                };
                self.positions
                    .set(self.stm.flip(), captured, captured_idx)?;
            } else {
                self.positions.set(self.stm.flip(), captured, to.index())?;
            }
        }

        // Unmake castling
        if move_data.is_castling {
            let (rook_from, rook_to) = match (self.stm, to.index()) {
                (Side::White, 6) => (5, 7),    // White kingside
                (Side::White, 2) => (3, 0),    // White queenside
                (Side::Black, 62) => (61, 63), // Black kingside
                (Side::Black, 58) => (59, 56), // Black queenside
                _ => unreachable!("[unmake_move] Invalid castling move"),
            };
            let rook_from_sq = Square::new(rook_from).unwrap();
            let rook_to_sq = Square::new(rook_to).unwrap();
            self.positions.update_piece_position(
                Piece::Rook,
                self.stm,
                rook_from_sq,
                rook_to_sq,
            )?;
        }

        self.calculate_material();
        Ok(())
    }

    // pub fn unmake_move(&mut self, move_data: &MoveInfo) -> miette::Result<()> {
    //     self.stm = self.stm.flip();
    //     self.positions.update_piece_position(
    //         move_data.piece_moved,
    //         self.stm,
    //         move_data.to,
    //         move_data.from,
    //     )?;
    //
    //     if let Some(captured) = move_data.captured_piece {
    //         if move_data.is_en_passant {
    //             // for en passant, the captured piece is not the 'to' square
    //             let captured_idx = match self.stm {
    //                 Side::White => move_data.to.index() - 8, // White made move, Black's pawn below
    //                 Side::Black => move_data.to.index() + 8, // Black made move, White's pawn above
    //             };
    //             self.positions
    //                 .set(self.stm.flip(), captured, captured_idx)?;
    //         } else {
    //             self.positions
    //                 .set(self.stm.flip(), captured, move_data.to.index())?;
    //         }
    //     }
    //
    //     if move_data.is_castling {
    //         let (rook_from, rook_to) = match (self.stm, move_data.to.index()) {
    //             (Side::White, 6) => (5, 7),    // White kingside
    //             (Side::White, 2) => (3, 0),    // White queenside
    //             (Side::Black, 62) => (61, 63), // Black kingside
    //             (Side::Black, 58) => (59, 56), // Black queenside
    //             _ => unreachable!("[unmake_move] Invalid castling move"),
    //         };
    //
    //         let rook_from_sq = Square::new(rook_from).unwrap();
    //         let rook_to_sq = Square::new(rook_to).unwrap();
    //
    //         self.positions.update_piece_position(
    //             Piece::Rook,
    //             self.stm,
    //             rook_from_sq,
    //             rook_to_sq,
    //         )?;
    //     }
    //
    //     self.castling_rights = move_data.castle_rights;
    //     self.enpassant_square = move_data.enpassant_square;
    //     self.halfmove_clock = move_data.halfmove_clock;
    //     if self.stm == Side::Black {
    //         self.fullmove_counter -= 1;
    //     }
    //     self.calculate_material();
    //
    //     Ok(())
    // }

    // pub fn is_move_legal(&self, from: Square, to: Square) -> bool {
    //     let piece = match self.get_piece_at(from) {
    //         Some(p) => p,
    //         None => {
    //             debug!("No piece at from");
    //             return false;
    //         }
    //     };
    //
    //     if !self.positions.square_belongs_to(self.stm, from.index()) {
    //         debug!("Piece on square {from} does not belong to {}", self.stm);
    //         return false;
    //     }
    //
    //     if piece == Piece::King {
    //         let file_diff = (to.col() as i32) - (from.col() as i32);
    //         if file_diff.abs() == 2 {
    //             let is_kingside = file_diff.is_positive();
    //
    //             let required_rights = match (self.stm, is_kingside) {
    //                 (Side::White, true) => CastlingRights(CastlingRights::WHITE_00),
    //                 (Side::White, false) => CastlingRights(CastlingRights::WHITE_000),
    //                 (Side::Black, true) => CastlingRights(CastlingRights::BLACK_00),
    //                 (Side::Black, false) => CastlingRights(CastlingRights::BLACK_000),
    //             };
    //
    //             if !self.castling_rights.allows(required_rights) {
    //                 debug!(
    //                     "Required rights {required_rights} not found. Current rights: {}",
    //                     self.castling_rights
    //                 );
    //                 return false;
    //             }
    //
    //             // Check if squares between king and rook are empty
    //             let rank = from.row();
    //             let start_file = from.col() as i32;
    //             let end_file = if is_kingside { 7 } else { 0 };
    //
    //             let (range_start, range_end) = if start_file < end_file {
    //                 (start_file + 1, end_file)
    //             } else {
    //                 (end_file + 1, start_file)
    //             };
    //             for file in range_start..range_end {
    //                 let square_idx = rank * 8 + file as usize;
    //                 if self.positions.is_occupied(square_idx) {
    //                     debug!("Path is blocked at {}", Square::new(square_idx).unwrap());
    //                     return false; // Path is blocked
    //                 }
    //             }
    //
    //             // Can't castle out of, through or into check
    //             let mut board_copy = *self;
    //             if board_copy.is_in_check(self.stm) {
    //                 debug!("{} is in check", self.stm);
    //                 return false;
    //             }
    //             // King passes through adjacent square
    //             let middle_square =
    //                 Square::new((from.index() as i32 + if is_kingside { 1 } else { -1 }) as usize)
    //                     .unwrap();
    //             board_copy
    //                 .positions
    //                 .update_piece_position(piece, self.stm, from, middle_square)
    //                 .unwrap_or_else(|e| debug!("Error in [is_move_legal]: {e}"));
    //
    //             if board_copy.is_in_check(self.stm) {
    //                 debug!("{} is in check afte passing through", self.stm);
    //                 return false;
    //             }
    //
    //             let mut board_copy = *self;
    //             board_copy.make_move(from, to);
    //
    //             return !board_copy.is_in_check(self.stm);
    //         }
    //     }
    //
    //     // Special handling for en passant
    //     if piece == Piece::Pawn
    //         && self.enpassant_square.is_some()
    //         && (to == self.enpassant_square.unwrap())
    //     {
    //         let file_diff = (to.col() as i32) - (from.col() as i32);
    //
    //         if file_diff.abs() == 1
    //             && ((self.stm == Side::White && to.row() - from.row() == 1)
    //                 || (self.stm == Side::Black && from.row() - to.row() == 1))
    //         {
    //             // Don't need to check if there's a piece at 'to' because enpassant square is
    //             // always empty
    //
    //             // Check if move leaves king in check
    //             let mut board_copy = *self;
    //             let captured_pawn_idx = match self.stm {
    //                 Side::White => to.index() - 8,
    //                 Side::Black => to.index() + 8,
    //             };
    //
    //             let _ =
    //                 board_copy
    //                     .positions
    //                     .capture(self.stm.flip(), Piece::Pawn, captured_pawn_idx);
    //
    //             return !board_copy.is_in_check(self.stm);
    //         }
    //     }
    //
    //     // Generate legal moves for the piece
    //     let moves = self.generate_legal_moves();
    //     if !moves.iter().any(|m| to == m.to_sq()) {
    //         debug!("{} cannot move from {} to {}", piece, from, to);
    //         return false;
    //     }
    //
    //     let mut board_copy = *self;
    //     board_copy.make_move(from, to);
    //     !board_copy.is_in_check(self.stm)
    // }

    /// To be used on a copy of the board.
    /// NOTE: Does not update side
    pub fn make_move(&mut self, m: Move) -> miette::Result<()> {
        let _ = self.make_move_with_info(m)?;
        Ok(())
    }

    pub fn make_move_with_info(&mut self, m: Move) -> miette::Result<MoveInfo> {
        let from = m.from_sq();
        let to = m.to_sq();
        let piece = self.get_piece_at(from).unwrap(); // Should always be a piece

        // Store current state for unmake
        let mut move_data = MoveInfo::new(from, to);
        move_data.piece_moved = piece;
        move_data.castle_rights = self.castling_rights;
        move_data.enpassant_square = self.enpassant_square;
        move_data.halfmove_clock = self.halfmove_clock;
        move_data.captured_piece = self.get_piece_at(to);

        // --- Update Board State ---
        self.halfmove_clock += 1;
        if m.is_capture() || piece == Piece::Pawn {
            self.halfmove_clock = 0;
        }

        // Reset en passant square
        self.enpassant_square = None;

        // Move the piece
        self.positions
            .update_piece_position(piece, self.stm, from, to)?;

        // Handle special move types
        match m.flags() {
            Move::DOUBLE_PAWN => {
                let ep_sq_idx = if self.stm == Side::White {
                    to.index() - 8
                } else {
                    to.index() + 8
                };
                self.enpassant_square = Some(Square::new(ep_sq_idx).unwrap());
            }
            Move::EN_PASSANT => {
                let captured_pawn_idx = if self.stm == Side::White {
                    to.index() - 8
                } else {
                    to.index() + 8
                };
                self.positions
                    .capture(self.stm.flip(), Piece::Pawn, captured_pawn_idx)?;
                move_data.captured_piece = Some(Piece::Pawn); // Set captured piece for unmake
            }
            Move::KING_CASTLE => {
                let (rook_from_file, rook_to_file) = (7, 5);
                let rank = from.row();
                let rook_from = Square::new(rank * 8 + rook_from_file).unwrap();
                let rook_to = Square::new(rank * 8 + rook_to_file).unwrap();
                self.positions
                    .update_piece_position(Piece::Rook, self.stm, rook_from, rook_to)?;
            }
            Move::QUEEN_CASTLE => {
                let (rook_from_file, rook_to_file) = (0, 3);
                let rank = from.row();
                let rook_from = Square::new(rank * 8 + rook_from_file).unwrap();
                let rook_to = Square::new(rank * 8 + rook_to_file).unwrap();
                self.positions
                    .update_piece_position(Piece::Rook, self.stm, rook_from, rook_to)?;
            }
            _ => {
                // Promotions
                if let Some(promo_piece) = m.promoted_piece() {
                    // Remove the pawn and add the new piece
                    self.positions.capture(self.stm, Piece::Pawn, to.index())?;
                    self.positions.set(self.stm, promo_piece, to.index())?;
                }
            }
        }

        // Update castling rights
        self.update_castling_rights(from);

        // Update clocks and side to move
        if self.stm == Side::Black {
            self.fullmove_counter += 1;
        }
        self.stm = self.stm.flip();
        self.calculate_material();

        Ok(move_data)
    }

    fn update_castling_rights(&mut self, from: Square) {
        match (self.stm, from.index()) {
            (Side::White, 4) => {
                // King moved
                self.castling_rights
                    .remove_right(&CastlingRights::WHITE_CASTLING);
            }
            (Side::White, 0) => {
                // A1 Rook moved
                self.castling_rights
                    .remove_right(&CastlingRights(CastlingRights::WHITE_000));
            }
            (Side::White, 7) => {
                // H1 Rook moved
                self.castling_rights
                    .remove_right(&CastlingRights(CastlingRights::WHITE_00));
            }
            (Side::Black, 60) => {
                // King moved
                self.castling_rights
                    .remove_right(&CastlingRights::BLACK_CASTLING);
            }
            (Side::Black, 56) => {
                // A8 Rook moved
                self.castling_rights
                    .remove_right(&CastlingRights(CastlingRights::BLACK_000));
            }
            (Side::Black, 63) => {
                // H8 Rook moved
                self.castling_rights
                    .remove_right(&CastlingRights(CastlingRights::BLACK_00));
            }
            _ => {}
        }
    }

    pub fn is_in_check(&self, side: Side) -> bool {
        let attack_data = calculate_attack_data(self, side);
        attack_data.in_check
    }

    pub fn is_checkmate(&self, side: Side) -> bool {
        self.is_in_check(side) && !self.generate_legal_moves().is_empty()
    }

    pub fn is_stalemate(&self, side: Side) -> bool {
        !self.is_in_check(side) && self.generate_legal_moves().is_empty()
    }

    pub fn is_draw(&self) -> bool {
        self.is_stalemate(self.stm) || self.halfmove_clock >= 100 || self.is_insufficient_material()
    }

    #[cfg(feature = "random")]
    pub fn suggest_rand_move(&self) -> miette::Result<(Square, Square)> {
        info!("This is RNGesus");
        let mut rng = rand::rng();
        let mut possible_end_bits: Vec<usize> = Vec::default();
        let mut from = Square::default();
        let mut to = Square::default();
        while possible_end_bits.is_empty() {
            // Choose a piece at random
            let (piece, _) = Piece::all()
                .choose(&mut rng)
                .expect("Should be able to choose at random");
            // Generate moves for the randomly selected piece
            let moves = MoveGen::new(piece, self.stm, self);

            // Get the position of the Piece on the current board
            let piece_state = self.positions.get_piece_bb(&self.stm, &piece);
            let piece_idx = piece_state.get_set_bits();

            let piece_choice = piece_idx
                .choose(&mut rng)
                .expect("Should be able to get a random piece idx");
            let m = moves.attack_bb[*piece_choice];
            possible_end_bits = m.get_set_bits();
            if possible_end_bits.is_empty() {
                continue;
            }
            from = Square::new(*piece_choice).expect("Should be valid piece choice");
            let end_bit = possible_end_bits
                .choose(&mut rng)
                .expect("Should be able to get random to square");
            to = Square::new(*end_bit).expect("Should be valid square");
        }
        Ok((from, to))
    }

    pub fn evaluate_position(&self, evaluator: &dyn Evaluator) -> i32 {
        evaluator.evaluate(self)
    }

    // NOTE: Older Implementation without support for full length FEN strings
    fn place_pieces(&mut self, fen: &str) -> miette::Result<()> {
        if fen.contains(' ') {
            return Err(miette::Error::msg("Not supported for now"));
        }
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
                    if let Some((piece, side)) = fen::PIECE_CHAR_LOOKUP_TABLE.get(&c) {
                        // dbg!(piece, side);
                        // dbg!(rank, file);
                        self.positions.set(*side, *piece, rank * 8 + file)?;
                        file += 1;
                    } else {
                        miette::bail!("Invalid Fen Character")
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        let index = square.index();

        for (piece_type, side) in Piece::all() {
            let piece_bb = self.positions.get_piece_bb(side, piece_type);
            if piece_bb.contains_square(index) {
                return Some(piece_type);
            }
        }
        None
    }

    pub fn handle_special_rules(&mut self, from: Square, to: Square) -> miette::Result<()> {
        let old_ep_square = self.enpassant_square;
        self.enpassant_square = None;

        match self.get_piece_at(from) {
            Some(Piece::Pawn) => {
                match old_ep_square {
                    // Enpassant capture
                    Some(ep_square) if to == ep_square => {
                        let captured_pawn_square = Square::new(match self.stm {
                            Side::White => ep_square.index() - 8,
                            Side::Black => ep_square.index() + 8,
                        })
                        .unwrap();

                        self.positions.capture(
                            self.stm.flip(),
                            Piece::Pawn,
                            captured_pawn_square.index(),
                        )?;
                    }
                    _ => {}
                }

                let is_double_push = match self.stm {
                    Side::White => from.index() / 8 == 1 && to.index() / 8 == 3,
                    Side::Black => from.index() / 8 == 6 && to.index() / 8 == 4,
                };

                if is_double_push {
                    let skipped_square = Square::new(match self.stm {
                        Side::White => from.index() + 8,
                        Side::Black => from.index() - 8,
                    })
                    .expect("Should be able to construct a valid skipped square");
                    self.enpassant_square = Some(skipped_square);
                }

                self.halfmove_clock = 0;
            }
            Some(Piece::King) => {
                match self.stm {
                    Side::White => self
                        .castling_rights
                        .remove_right(&CastlingRights::WHITE_CASTLING),
                    Side::Black => self
                        .castling_rights
                        .remove_right(&CastlingRights::BLACK_CASTLING),
                }
                let file_diff = (to.col() as i8) - (from.col() as i8);
                if file_diff.abs() == 2 {
                    // this is a castling move
                    let rook_from_file = if file_diff.is_positive() { 7 } else { 0 }; // kingside or queenside A1/H1
                    let rook_to_file = if file_diff.is_positive() { 5 } else { 3 }; // kingside or queenside C1/F1

                    let rank = from.row();
                    let rook_from = Square::new(rank * 8 + rook_from_file).unwrap();
                    let rook_to = Square::new(rank * 8 + rook_to_file).unwrap();

                    self.positions.update_piece_position(
                        Piece::Rook,
                        self.stm,
                        rook_from,
                        rook_to,
                    )?;
                }
            }
            Some(Piece::Rook) => match (self.stm, from.index()) {
                (Side::White, 0) => self
                    .castling_rights
                    .remove_right(&CastlingRights(CastlingRights::WHITE_000)),
                (Side::White, 7) => self
                    .castling_rights
                    .remove_right(&CastlingRights(CastlingRights::WHITE_00)),
                (Side::Black, 56) => self
                    .castling_rights
                    .remove_right(&CastlingRights(CastlingRights::BLACK_000)),
                (Side::Black, 63) => self
                    .castling_rights
                    .remove_right(&CastlingRights(CastlingRights::BLACK_00)),
                _ => {}
            },
            None => {
                miette::bail!("No piece at from ({from}) square");
            }
            _ => {}
        }
        Ok(())
    }

    fn calculate_material(&mut self) {
        // Reset material counts
        self.material = [0; 2];

        for side in [Side::White, Side::Black] {
            let side_index = side.index();
            for piece in Piece::colored_pieces(side) {
                let piece_bb = self.positions.get_piece_bb(side, piece);
                let piece_count = piece_bb.0.count_ones() as u64;
                let piece_value: u64 = piece.value().into();
                self.material[side_index] += piece_count * piece_value;
            }
        }
    }

    fn is_insufficient_material(&self) -> bool {
        let white_pieces = self.positions.get_side_bb(Side::White);
        let black_pieces = self.positions.get_side_bb(Side::Black);

        // Arrays to store the counts of each piece type
        let mut white_counts = [0; 6];
        let mut black_counts = [0; 6];

        // Count the pieces for both sides
        for piece in Piece::PIECES.iter() {
            white_counts[piece.index()] =
                self.positions.get_piece_bb(Side::White, *piece).pop_count();

            black_counts[piece.index()] =
                self.positions.get_piece_bb(Side::Black, *piece).pop_count();
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
        let white_bishop = self.positions.get_piece_bb(Side::White, Piece::Bishop);
        let black_bishop = self.positions.get_piece_bb(Side::Black, Piece::Bishop);

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

    use std::str::FromStr;

    use crate::init;

    use super::*;

    #[test]
    fn test_make_unmake_move() {
        init();
        let mut board = Board::new();
        let orig_board = board;

        let from = Square::from_str("e2").unwrap();
        let to = Square::from_str("e4").unwrap();

        let move_data = board.try_move_with_info(from, to).unwrap();

        assert_ne!(board, orig_board);

        board.unmake_move(&move_data).unwrap();

        assert_eq!(board, orig_board);
    }

    #[test]
    fn test_make_unmake_capture() {
        init();
        let mut board =
            Board::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1");
        let orig_board = board;
        println!("{board}");

        let from = Square::from_str("e4").unwrap();
        let to = Square::from_str("d5").unwrap();

        let move_data = board.try_move_with_info(from, to).unwrap();

        assert_ne!(board, orig_board);

        board.unmake_move(&move_data).unwrap();

        assert_eq!(board, orig_board);
    }
    #[test]
    fn test_initial_material_balance() {
        let mut board = Board::new();
        board.calculate_material();
        // Initial material for each side should be:
        // 8 pawns (8 * 100 = 800)
        // 2 knights (2 * 320 = 640)
        // 2 bishops (2 * 330 = 660)
        // 2 rooks (2 * 500 = 1000)
        // 1 queen (1 * 900 = 900)
        // 1 king (1 * 20000 = 20000)
        // Total: 24000
        assert_eq!(board.material[Side::White.index()], 24000);
        assert_eq!(board.material[Side::Black.index()], 24000);
    }

    #[test]
    fn test_material_after_capture() {
        let mut board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKBNR w KQkq - 0 1");
        board.calculate_material();
        // Standard position with a missing f2 pawn on white's side

        assert_eq!(board.material[Side::White.index()], 23900); // 24000 - 100 = 23900
        assert_eq!(board.material[Side::Black.index()], 24000);
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
