use crate::{
    START_FEN,
    evaluation::Evaluator,
    moves::{
        attack_data::calculate_attack_data,
        move_gen::{generate_legal_captures, generate_legal_moves, generate_pseudo_legal_moves},
        move_info::{Move, MoveInfo},
        precomputed::MOVE_TABLES,
    },
    zobrist::{ZOBRIST, calculate_hash},
};
use miette::Context;
#[cfg(feature = "random")]
use rand::prelude::*;
use std::fmt::Display;

use self::components::{BoardState, CastlingRights, Piece, Side, Square};

pub mod components;
pub mod fen;
#[cfg(test)]
mod tests;
pub mod zobrist;

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
    pub material: [u32; 2],
    /// Zobrist hash
    pub hash: u64,
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
        Board::from_fen(START_FEN)
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
        board.recalculate_material();
        board.hash = calculate_hash(&board);
        board
    }

    pub fn to_fen(&self) -> miette::Result<String> {
        fen::to_fen(self)
    }

    pub fn generate_legal_moves(&self, captures_only: bool) -> Vec<Move> {
        let mut legal_moves = Vec::with_capacity(40);
        if !captures_only {
            generate_legal_moves(self, &mut legal_moves);
        } else {
            generate_legal_captures(self, &mut legal_moves);
        }
        legal_moves
    }

    pub fn generate_pseudo_legal_moves(&self) -> Vec<Move> {
        let mut pseudo_legal_moves = Vec::with_capacity(40);
        generate_pseudo_legal_moves(
            &self.positions,
            self.stm,
            self.castling_rights,
            self.enpassant_square,
            &mut pseudo_legal_moves,
        );
        pseudo_legal_moves
    }

    /// Primary "safe" method for applying a move.
    /// Checks for legality before making the move.
    pub fn try_move(&mut self, m: Move) -> miette::Result<()> {
        let legal_moves = self.generate_legal_moves(false);

        if legal_moves.contains(&m) {
            let _ = self.make_move(m)?;
            Ok(())
        } else {
            miette::bail!("Illegal move from {}", m);
        }
    }

    pub fn unmake_move(&mut self, move_data: &MoveInfo) -> miette::Result<()> {
        self.stm = self.stm.flip();
        self.castling_rights = move_data.castle_rights;
        self.enpassant_square = move_data.enpassant_square;
        self.halfmove_clock = move_data.halfmove_clock;
        self.hash = move_data.zobrist_hash;
        if self.stm == Side::Black {
            self.fullmove_counter -= 1;
        }

        let from = move_data.from;
        let to = move_data.to;
        let opponent = self.stm.flip();
        let _piece_moved = move_data.piece_moved;

        // Restore moved piece
        if let Some(promoted_piece) = move_data.promotion {
            self.positions
                .remove_piece(self.stm, promoted_piece, to.index())?;
            self.positions.set(self.stm, Piece::Pawn, from.index())?;
        } else {
            self.positions.move_piece(to, from)?;
        }

        // Restore captured pieces
        if let Some(captured) = move_data.captured_piece {
            if move_data.is_en_passant {
                let captured_idx = match self.stm {
                    Side::White => to.index() - 8,
                    Side::Black => to.index() + 8,
                };
                self.positions.set(opponent, captured, captured_idx)?;
            } else {
                self.positions.set(opponent, captured, to.index())?;
            }
        }

        // Unmake castling (Moving rook)
        if move_data.is_castling {
            let (rook_from, rook_to) = match (self.stm, to.index()) {
                (Side::White, 6) => (5, 7),    // White kingside: rook from f1 to h1
                (Side::White, 2) => (3, 0),    // White queenside: rook from d1 to a1
                (Side::Black, 62) => (61, 63), // Black kingside: rook from f8 to h8
                (Side::Black, 58) => (59, 56), // Black queenside: rook from d8 to a8
                _ => unreachable!("[unmake_move] Invalid castling move data"),
            };
            let rook_from_sq = Square::new(rook_from).unwrap();
            let rook_to_sq = Square::new(rook_to).unwrap();
            self.positions.move_piece(rook_from_sq, rook_to_sq)?;
        }

        self.material = move_data.material;
        Ok(())
    }

    pub fn make_null_move(&mut self) {
        self.stm = self.stm.flip();
    }

    /// The primary "unsafe" but fast method for applying a move.
    /// It assumes the move is legal and comes from our own generator.
    /// Also updates zobrist hash
    /// Returns the `MoveInfo` needed to unmake the move.
    pub fn make_move(&mut self, m: Move) -> miette::Result<MoveInfo> {
        let from = m.from_sq();
        let to = m.to_sq();
        let piece = self
            .get_piece_at(from)
            .context("A piece should exist at from sq")?;
        let opponent = self.stm.flip();

        // Store current state for unmake
        let move_data = MoveInfo {
            from,
            to,
            piece_moved: piece,
            castle_rights: self.castling_rights,
            enpassant_square: self.enpassant_square,
            halfmove_clock: self.halfmove_clock,
            zobrist_hash: self.hash,
            is_castling: m.is_castling(),
            is_en_passant: m.is_enpassant(),
            promotion: m.promoted_piece(),
            material: self.material,
            captured_piece: if m.is_enpassant() {
                Some(Piece::Pawn)
            } else {
                self.get_piece_at(to)
            },
        };

        if let Some(ep_sq) = self.enpassant_square {
            self.hash ^= ZOBRIST.en_passant_file[ep_sq.col()];
        }
        self.enpassant_square = None;
        // XOR out prev castling rights.
        self.hash ^= ZOBRIST.castling[self.castling_rights.0 as usize];

        // Update Board State
        // This covers normal captures and promotion-captures.
        // Enpassant capture is handled further down
        if let Some(captured_piece) = move_data.captured_piece
            && !m.is_enpassant()
        {
            self.positions
                .remove_piece(opponent, captured_piece, to.index())?;
            // XOR out key for removed piece
            self.hash ^= ZOBRIST.pieces[opponent.index()][captured_piece.index()][to.index()];
            self.material[opponent.index()] -= captured_piece.value();
        }

        // Move the piece from 'from' to 'to'
        self.positions.move_piece(from, to)?;
        // XOR out key for moved piece at source sq 'from'
        self.hash ^= ZOBRIST.pieces[self.stm.index()][piece.index()][from.index()];
        // XOR in key for moved piece at destination sq 'to'
        self.hash ^= ZOBRIST.pieces[self.stm.index()][piece.index()][to.index()];

        match m.flags() {
            Move::DOUBLE_PAWN => {
                let ep_sq_idx = if self.stm == Side::White {
                    to.index() - 8
                } else {
                    to.index() + 8
                };
                let opponent_pawns = self.positions.get_piece_bb(opponent, Piece::Pawn);
                let legal_ep_attackers = MOVE_TABLES.get_pawn_attacks(ep_sq_idx, self.stm);
                if (*opponent_pawns & legal_ep_attackers).any() {
                    self.enpassant_square = Square::new(ep_sq_idx);
                    // XOR in new enpassant file.
                    self.hash ^= ZOBRIST.en_passant_file[ep_sq_idx % 8];
                }
            }
            Move::EN_PASSANT => {
                let captured_pawn_idx = if self.stm == Side::White {
                    to.index() - 8
                } else {
                    to.index() + 8
                };
                self.positions
                    .remove_piece(opponent, Piece::Pawn, captured_pawn_idx)?;
                // XOR out captured opponent pawn
                self.hash ^= ZOBRIST.pieces[opponent.index()][Piece::pawn()][captured_pawn_idx];
                self.material[opponent.index()] -= Piece::Pawn.value();
            }
            Move::KING_CASTLE => {
                let (rook_from, rook_to) = (
                    Square::new(from.row() * 8 + 7).unwrap(),
                    Square::new(from.row() * 8 + 5).unwrap(),
                );
                self.positions.move_piece(rook_from, rook_to)?;
                // XOR out rook from source sq
                self.hash ^= ZOBRIST.pieces[self.stm.index()][Piece::rook()][rook_from.index()];
                // XOR in rook from destination sq
                self.hash ^= ZOBRIST.pieces[self.stm.index()][Piece::rook()][rook_to.index()];
            }
            Move::QUEEN_CASTLE => {
                let (rook_from, rook_to) = (
                    Square::new(from.row() * 8).unwrap(),
                    Square::new(from.row() * 8 + 3).unwrap(),
                );
                self.positions.move_piece(rook_from, rook_to)?;
                // XOR out rook from source sq
                self.hash ^= ZOBRIST.pieces[self.stm.index()][Piece::rook()][rook_from.index()];
                // XOR in rook from destination sq
                self.hash ^= ZOBRIST.pieces[self.stm.index()][Piece::rook()][rook_to.index()];
            }
            _flags if m.is_promotion() => {
                let promo_piece = m.promoted_piece().unwrap();
                // The pawn is already at the 'to' square, so we replace it.
                self.positions
                    .remove_piece(self.stm, Piece::Pawn, to.index())?;
                self.material[self.stm.index()] -= Piece::Pawn.value();
                // XOR out pawn
                self.hash ^= ZOBRIST.pieces[self.stm.index()][Piece::pawn()][to.index()];

                self.positions.set(self.stm, promo_piece, to.index())?;
                self.material[self.stm.index()] += promo_piece.value();
                // XOR in promote piece
                self.hash ^= ZOBRIST.pieces[self.stm.index()][promo_piece.index()][to.index()];
            }
            _ => { /* Quiet and normal captures fall through to here,
                but they dont need anything special */
            }
        }

        // Final state update
        self.update_castling_rights(from, to);
        // XOR in updated castling rights;
        self.hash ^= ZOBRIST.castling[self.castling_rights.0 as usize];

        if piece == Piece::Pawn || m.is_capture() {
            self.halfmove_clock = 0
        } else {
            self.halfmove_clock += 1;
        }

        // Flip side to move
        if self.stm == Side::Black {
            self.fullmove_counter += 1;
        }
        self.stm = opponent;
        // XOR side
        self.hash ^= ZOBRIST.black_to_move;

        Ok(move_data)
    }

    fn update_castling_rights(&mut self, from: Square, to: Square) {
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

        match to.index() {
            0 => self
                .castling_rights
                .remove_right(&CastlingRights(CastlingRights::WHITE_000)), // White's A1 rook captured
            7 => self
                .castling_rights
                .remove_right(&CastlingRights(CastlingRights::WHITE_00)), // White's H1 rook captured
            56 => self
                .castling_rights
                .remove_right(&CastlingRights(CastlingRights::BLACK_000)), // Black's A8 rook captured
            63 => self
                .castling_rights
                .remove_right(&CastlingRights(CastlingRights::BLACK_00)), // Black's H8 rook captured
            _ => {}
        }
    }

    pub fn is_in_check(&self, side: Side) -> bool {
        let attack_data = calculate_attack_data(self, side);
        attack_data.in_check
    }

    pub fn is_checkmate(&self, side: Side) -> bool {
        self.is_in_check(side) && !self.generate_legal_moves(false).is_empty()
    }

    pub fn is_stalemate(&self, side: Side) -> bool {
        !self.is_in_check(side) && self.generate_legal_moves(false).is_empty()
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

    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        self.positions.get_piece_at(&square).map(|(piece, _)| piece)
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

                        self.positions.remove_piece(
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

                    self.positions.move_piece(rook_from, rook_to)?;
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

    fn recalculate_material(&mut self) {
        // Reset material
        self.material = [0; 2];
        for side in [Side::White, Side::Black] {
            let side_index = side.index();
            for piece in Piece::colored_pieces(side) {
                let piece_bb = self.positions.get_piece_bb(side, piece);
                let piece_count = piece_bb.0.count_ones();
                let piece_value = piece.value();
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
        Piece::all_pieces().for_each(|piece| {
            white_counts[piece.index()] =
                self.positions.get_piece_bb(Side::White, piece).pop_count();

            black_counts[piece.index()] =
                self.positions.get_piece_bb(Side::Black, piece).pop_count();
        });

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
            white_bishop.iter_bits().next(),
            black_bishop.iter_bits().next(),
        ) {
            // Bishops are on same colored squares if sum of their coordinates is even/odd same
            let (white_file, white_rank) = Square::new(white_sq).unwrap().coords();
            let (black_file, black_rank) = Square::new(black_sq).unwrap().coords();

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
        let mov = Move::new(from.index() as u8, to.index() as u8, Move::QUIET);
        let orig_mat = board.material;

        let move_data = board.make_move(mov).unwrap();

        let moved_mat = board.material;

        assert_ne!(board, orig_board);
        assert_eq!(orig_mat, moved_mat);

        board.unmake_move(&move_data).unwrap();
        let restored_mat = board.material;

        assert_eq!(board, orig_board);
        assert_eq!(orig_mat, restored_mat);
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
        let mov = Move::new(from.index() as u8, to.index() as u8, Move::CAPTURE);
        let orig_mat = board.material;

        let move_data = board.make_move(mov).unwrap();
        let moved_mat = board.material;

        assert_ne!(board, orig_board);
        assert_ne!(orig_mat, moved_mat);

        board.unmake_move(&move_data).unwrap();
        let restored_mat = board.material;

        println!("original board: \n{orig_board}");
        println!("unmade board: \n{board}");
        assert_eq!(board, orig_board);
        assert_eq!(orig_mat, restored_mat);
    }
    #[test]
    fn test_initial_material_balance() {
        let mut board = Board::new();
        board.recalculate_material();
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
        board.recalculate_material();
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
