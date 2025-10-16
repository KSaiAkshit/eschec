use crate::{
    board::zobrist::calculate_hash,
    evaluation::score::Phase,
    moves::{attack_data::calculate_attack_data, move_gen},
    prelude::*,
};
use miette::Context;
use std::fmt::Display;

pub mod components;
pub mod fen;
#[cfg(test)]
mod tests;
pub mod zobrist;

/// Completely encapsulate the game
#[derive(Default, Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Copy)]
pub struct Board {
    /// Snapshot of current board
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
    pub material: [Score; 2],
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
            "Material balance: W: {} B: {}",
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

    pub fn generate_legal_moves(&self, buffer: &mut MoveBuffer, forcing_only: bool) {
        if !forcing_only {
            move_gen::generate_legal_moves::<move_gen::AllMoves>(self, buffer);
        } else {
            move_gen::generate_legal_moves::<move_gen::ForcingMoves>(self, buffer);
        }
    }

    /// Only to be used internally;
    fn get_legal_moves(&self, captures_only: bool) -> MoveBuffer {
        let mut buffer = MoveBuffer::new();
        self.generate_legal_moves(&mut buffer, captures_only);
        buffer
    }

    pub fn generate_pseudo_legal_moves(&self, buffer: &mut MoveBuffer) {
        move_gen::generate_pseudo_legal_moves(
            &self.positions,
            self.stm,
            self.castling_rights,
            self.enpassant_square,
            buffer,
        );
    }

    /// Primary "safe" method for applying a move.
    /// Checks for legality before making the move.
    pub fn try_move(&mut self, m: Move) -> miette::Result<()> {
        let mut legal_moves = MoveBuffer::new();
        self.generate_legal_moves(&mut legal_moves, false);

        if legal_moves.contains(&m) {
            let _ = self.make_move(m)?;
            Ok(())
        } else {
            let mut possible_moves = String::new();
            for mv in legal_moves {
                possible_moves.push_str(&mv.uci());
                possible_moves.push(' ');
            }
            println!("Possibe moves: {possible_moves}");
            miette::bail!("Illegal move for {} from {}", &self.stm, m.uci());
        }
    }

    /// Method to unmake a move, but arguably, making a Copy of the board to make
    /// a Move is and discarding the Copy later is faster.
    pub fn unmake_move(&mut self, move_data: &MoveInfo) -> miette::Result<()> {
        self.stm = move_data.stm;
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
            self.material[self.stm.index()] -= promoted_piece.score();
            self.material[self.stm.index()] += Piece::Pawn.score();
            self.positions
                .remove_piece(self.stm, promoted_piece, to.index())?;
            self.positions.set(self.stm, Piece::Pawn, from.index())?;
        } else {
            self.positions.move_piece(to, from)?;
        }

        // Restore captured pieces
        if let Some(captured) = move_data.captured_piece {
            self.material[opponent.index()] += captured.score();
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

        // self.material = move_data.material;
        Ok(())
    }

    pub fn make_null_move(&mut self) {
        if let Some(ep_square) = self.enpassant_square {
            self.hash ^= ZOBRIST.en_passant_file[ep_square.col()];
        }
        self.enpassant_square = None;

        self.stm = self.stm.flip();
        self.hash ^= ZOBRIST.black_to_move;
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
            .with_context(|| format!("A piece should exist at {from} sq"))?;
        let opponent = self.stm.flip();

        // Store current state for unmake
        let move_data = MoveInfo {
            from,
            to,
            stm: self.stm,
            piece_moved: piece,
            castle_rights: self.castling_rights,
            enpassant_square: self.enpassant_square,
            halfmove_clock: self.halfmove_clock,
            zobrist_hash: self.hash,
            is_castling: m.is_castling(),
            is_en_passant: m.is_enpassant(),
            promotion: m.promoted_piece(),
            // material: self.material,
            captured_piece: if m.is_enpassant() {
                Some(Piece::Pawn)
            } else {
                self.get_piece_at(to)
            },
        };

        if let Some(ep_sq) = self.enpassant_square {
            self.hash ^= &ZOBRIST.en_passant_file[ep_sq.col()];
        }
        self.enpassant_square = None;
        // XOR out prev castling rights.
        self.hash ^= &ZOBRIST.castling[self.castling_rights.get_rights() as usize];

        // Update Board State
        // This covers normal captures and promotion-captures.
        // Enpassant capture is handled further down
        if let Some(captured_piece) = move_data.captured_piece
            && !m.is_enpassant()
        {
            self.positions
                .remove_piece(opponent, captured_piece, to.index())?;
            // XOR out key for removed piece
            self.hash ^= &ZOBRIST.pieces[opponent.index()][captured_piece.index()][to.index()];
            self.material[opponent.index()] -= captured_piece.score();
        }

        // Move the piece from 'from' to 'to'
        self.positions.move_piece(from, to)?;
        // XOR out key for moved piece at source sq 'from'
        self.hash ^= &ZOBRIST.pieces[self.stm.index()][piece.index()][from.index()];
        // XOR in key for moved piece at destination sq 'to'
        self.hash ^= &ZOBRIST.pieces[self.stm.index()][piece.index()][to.index()];

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
                    self.hash ^= &ZOBRIST.en_passant_file[ep_sq_idx % 8];
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
                self.hash ^= &ZOBRIST.pieces[opponent.index()][Piece::pawn()][captured_pawn_idx];
                self.material[opponent.index()] -= Piece::Pawn.score();
            }
            Move::KING_CASTLE => {
                let (rook_from, rook_to) = (
                    Square::new(from.row() * 8 + 7).unwrap(),
                    Square::new(from.row() * 8 + 5).unwrap(),
                );
                self.positions.move_piece(rook_from, rook_to)?;
                // XOR out rook from source sq
                self.hash ^= &ZOBRIST.pieces[self.stm.index()][Piece::rook()][rook_from.index()];
                // XOR in rook from destination sq
                self.hash ^= &ZOBRIST.pieces[self.stm.index()][Piece::rook()][rook_to.index()];
                self.castling_rights.set_castled(self.stm);
            }
            Move::QUEEN_CASTLE => {
                let (rook_from, rook_to) = (
                    Square::new(from.row() * 8).unwrap(),
                    Square::new(from.row() * 8 + 3).unwrap(),
                );
                self.positions.move_piece(rook_from, rook_to)?;
                // XOR out rook from source sq
                self.hash ^= &ZOBRIST.pieces[self.stm.index()][Piece::rook()][rook_from.index()];
                // XOR in rook from destination sq
                self.hash ^= &ZOBRIST.pieces[self.stm.index()][Piece::rook()][rook_to.index()];
            }
            _flags if m.is_promotion() => {
                let promo_piece = m.promoted_piece().unwrap();
                // The pawn is already at the 'to' square, so we replace it.
                self.positions
                    .remove_piece(self.stm, Piece::Pawn, to.index())?;
                self.material[self.stm.index()] -= Piece::Pawn.score();
                // XOR out pawn
                self.hash ^= &ZOBRIST.pieces[self.stm.index()][Piece::pawn()][to.index()];

                self.positions.set(self.stm, promo_piece, to.index())?;
                self.material[self.stm.index()] += promo_piece.score();
                // XOR in promote piece
                self.hash ^= &ZOBRIST.pieces[self.stm.index()][promo_piece.index()][to.index()];
            }
            _ => { /* Quiet and normal captures fall through to here,
                but they dont need anything special */
            }
        }

        // Final state update
        self.update_castling_rights(from, to);
        // XOR in updated castling rights;
        self.hash ^= &ZOBRIST.castling[self.castling_rights.get_rights() as usize];

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
        self.hash ^= &ZOBRIST.black_to_move;

        Ok(move_data)
    }

    pub fn static_exchange_evaluation(&self, mv: Move) -> i32 {
        let from_sq = mv.from_sq();
        let to_sq = mv.to_sq();
        let mut side_to_move = self.stm;

        let mut occupied = self.positions.get_occupied_bb();
        let mut gain = [0; 32];
        let mut final_gain_idx = 0;

        let attacker_piece = match self.get_piece_at(from_sq) {
            Some(p) => p,
            None => unreachable!(
                "Move is supposed to be legal.
                There should be an attacker_piece at {from_sq}."
            ),
        };
        let victim_piece = if mv.is_enpassant() {
            Piece::Pawn
        } else {
            match self.get_piece_at(to_sq) {
                Some(p) => p,
                None => return 0,
            }
        };

        gain[final_gain_idx] = victim_piece.victim_score();
        final_gain_idx += 1;
        // Attacker becomes the next victim
        gain[final_gain_idx] = attacker_piece.victim_score() - gain[final_gain_idx - 1];

        occupied.capture(from_sq.index());
        if mv.is_enpassant() {
            let captured_pawn_sq_idx = if side_to_move == Side::White {
                to_sq.get_neighbor(Direction::SOUTH).index()
            } else {
                to_sq.get_neighbor(Direction::NORTH).index()
            };
            occupied.capture(captured_pawn_sq_idx);
        }
        // Redundant: No need to capture/set bit again.
        // else {
        //     occupied.capture(to_sq.index());
        // }
        // occupied.set(to_sq.index());

        side_to_move = side_to_move.flip();

        loop {
            final_gain_idx += 1;
            if final_gain_idx >= gain.len() {
                break;
            }
            let attackers_bb = move_gen::get_attackers_to(self, to_sq, side_to_move, occupied);

            let mut lva_piece = None;
            let mut lva_from_sq = None;

            // NOTE: Here, lva is found by iterating from low value to high value pieces and
            // breaking the look when any piece is found to be a potential attacker
            for piece in Piece::all_pieces() {
                let lva_candidates =
                    attackers_bb & *self.positions.get_piece_bb(side_to_move, piece);
                if lva_candidates.any() {
                    lva_piece = Some(piece);
                    lva_from_sq = Square::new(lva_candidates.lsb().unwrap() as usize);
                    break;
                }
            }

            if let (Some(piece), Some(from)) = (lva_piece, lva_from_sq) {
                gain[final_gain_idx] = piece.victim_score() - gain[final_gain_idx - 1];

                occupied.capture(from.index());
                side_to_move = side_to_move.flip();
            } else {
                // No more attackers
                break;
            }
        }

        // Last capture is a 'speculative' store,
        // so this doesn't actually happen because there are not attackers.
        // It is right to ignore this since it is 'speculative'
        final_gain_idx -= 1;
        while final_gain_idx > 1 {
            final_gain_idx -= 1;
            gain[final_gain_idx - 1] = gain[final_gain_idx - 1].min(-gain[final_gain_idx]);
        }

        gain[0]
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

    pub fn game_phase(&self) -> Phase {
        let mut phase = TOTAL_PHASE;

        let mut i = 0;
        while i < 6 {
            let piece = Piece::PIECES[i];
            phase -=
                self.positions.get_piece_bb(Side::White, piece).pop_count() as i32 * piece.phase();
            phase -=
                self.positions.get_piece_bb(Side::Black, piece).pop_count() as i32 * piece.phase();
            i += 1;
        }

        let scaled_phase = (phase * ENDGAME_PHASE + (TOTAL_PHASE / 2)) / TOTAL_PHASE;

        Phase(scaled_phase)
    }

    pub fn is_in_check(&self, side: Side) -> bool {
        let attack_data = calculate_attack_data(self, side);
        attack_data.in_check
    }

    pub fn is_checkmate(&self, side: Side) -> bool {
        self.is_in_check(side) && self.get_legal_moves(false).is_empty()
    }

    pub fn is_stalemate(&self, side: Side) -> bool {
        !self.is_in_check(side) && self.get_legal_moves(false).is_empty()
    }

    pub fn is_draw(&self) -> bool {
        self.is_stalemate(self.stm) || self.halfmove_clock >= 100 || self.is_insufficient_material()
    }

    pub fn evaluate_position(&self, evaluator: &dyn Evaluator) -> i32 {
        let phase = self.game_phase();
        evaluator.evaluate(self).taper(phase)
    }

    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        self.positions.get_piece_at(&square).map(|(piece, _)| piece)
    }

    fn recalculate_material(&mut self) {
        // Reset material
        self.material = [Score::default(); 2];
        for side in [Side::White, Side::Black] {
            let side_index = side.index();
            for piece in Piece::colored_pieces(side) {
                let piece_bb = self.positions.get_piece_bb(side, piece);
                let piece_count = piece_bb.0.count_ones();
                let piece_value = piece.score();
                self.material[side_index] += piece_value * piece_count as i32;
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
        if white_pieces.pop_count() == 1 && black_pieces.pop_count() == 1 {
            return true;
        }

        // TODO: change all count_ones to pop_count. The logic is wrong!

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
