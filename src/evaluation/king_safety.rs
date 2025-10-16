use crate::prelude::*;

#[rustfmt::skip]
const SAFETY_TABLE: [i32; 100] = [
    0, 0, 1, 2, 3, 5, 7, 9, 12, 15,
    18, 22, 26, 30, 35, 39, 44, 50, 56, 62,
    68, 75, 82, 85, 89, 97, 105, 113, 122, 131,
    140, 150, 169, 180, 191, 202, 213, 225, 237, 248,
    260, 272, 283, 295, 307, 319, 330, 342, 354, 366,
    377, 389, 401, 412, 424, 436, 448, 459, 471, 483,
    494, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500
];

#[derive(Debug, Clone)]
pub struct KingSafetyEvaluator {
    name: String,
    // Bonus
    /// Bonus per castling option available
    castling_bonus: i32,
    pawn_shield_bonus_full: i32,
    pawn_shield_bonus_partial: i32,

    // Penalties
    open_file_penalty: i32,
}

impl Default for KingSafetyEvaluator {
    fn default() -> Self {
        Self {
            name: "KingSafety".to_string(),
            castling_bonus: 0,
            pawn_shield_bonus_full: 0,
            pawn_shield_bonus_partial: 0,
            open_file_penalty: 0,
        }
    }
}

impl KingSafetyEvaluator {
    pub fn new() -> Self {
        Self {
            name: "KingSafety".to_string(),
            castling_bonus: 10,
            pawn_shield_bonus_full: 30,    // Bonus for full pawn shield
            pawn_shield_bonus_partial: 10, // Bonus for partial pawn shield
            open_file_penalty: -40,        // Strong penalty for open files
        }
    }
    fn evaluate_castling(&self, rights: CastlingRights, side: Side) -> i32 {
        let mut score = 0;

        if rights.has_castled(side) {
            score += self.castling_bonus;

            return score;
        }

        // Subtracting some score here so that having two rights
        // is worth less than having castled.
        if rights.can_castle(side, true) {
            score += (self.castling_bonus / 2) - 2;
        }

        if rights.can_castle(side, false) {
            score += (self.castling_bonus / 2) - 2;
        }

        score
    }

    fn evaluate_pawn_shield(&self, board: &Board, side: Side, king_sq: usize) -> i32 {
        let friendly_pawns = board.positions.get_piece_bb(side, Piece::Pawn);
        let mut shield_score = 0;

        let king_file = king_sq % 8;
        let king_rank = king_sq / 8;

        let shield_rank1_mask = PAWN_TABLES.king_shield_zone_masks[side.index()][king_sq];
        let shield_rank2_mask =
            if (side == Side::White && king_rank < 6) || (side == Side::Black && king_rank > 1) {
                let square_ahead = king_sq as i32 + (if side == Side::White { 8 } else { -8 });

                PAWN_TABLES.king_shield_zone_masks[side.index()][square_ahead as usize]
            } else {
                BitBoard(0)
            };
        let total_shield_zone = shield_rank1_mask | shield_rank2_mask;

        let shielded_pawns_in_zone = friendly_pawns & &total_shield_zone;

        let mut full_shield_files = 0;
        for file_offset in [-1, 0, 1] {
            let target_file_idx = king_file as i8 + file_offset;
            if (0..8).contains(&target_file_idx)
                && (shielded_pawns_in_zone & BitBoard(FILE_MASKS[target_file_idx as usize])).any()
            {
                full_shield_files += 1;
            }
        }

        match full_shield_files {
            3 => shield_score += self.pawn_shield_bonus_full,
            2 => shield_score += self.pawn_shield_bonus_partial,
            _ => {}
        }
        shield_score
    }

    fn evaluate_open_files(&self, board: &Board, side: Side, king_sq: usize) -> i32 {
        let king_file = king_sq % 8;
        let opponent = side.flip();
        let opponent_pawns = board.positions.get_piece_bb(opponent, Piece::Pawn);
        let friendly_pawns = board.positions.get_piece_bb(side, Piece::Pawn);

        let mut penalty = 0;

        for file_offset in [-1, 0, 1] {
            let target_file_idx = king_file as i8 + file_offset;
            if (0..8).contains(&target_file_idx) {
                let target_file_mask = BitBoard(FILE_MASKS[target_file_idx as usize]);

                // A file is "open" if no pawns of either side are on it.
                // A file is "semi-open" if no friendly pawns are on it, but opponent pawns are.
                let has_friendly_pawns_on_file = (target_file_mask & *friendly_pawns).any();
                let has_opponent_pawns_on_file = (target_file_mask & *opponent_pawns).any();

                if has_friendly_pawns_on_file {
                    continue;
                }
                let opp_rooks_queens = board.positions.get_orhto_sliders_bb(opponent);
                if (target_file_mask & opp_rooks_queens).any() {
                    if has_opponent_pawns_on_file {
                        // Semi open file
                        penalty += self.open_file_penalty / 2;
                    } else {
                        penalty += self.open_file_penalty;
                    }
                }
            }
        }

        penalty
    }

    fn calculate_attack_units(&self, board: &Board, side: Side, king_sq: usize) -> i32 {
        let opponent = side.flip();
        let mut total_attack_units = 0;

        let king_zone_mask = PAWN_TABLES.king_attack_zone_masks[side.index()][king_sq];

        let occupied_for_attacks = board.positions.get_occupied_bb();

        for piece_type in Piece::all_pieces() {
            if piece_type == Piece::King {
                continue;
            }

            let mut opponent_pieces_bb = *board.positions.get_piece_bb(opponent, piece_type);
            while let Some(from_sq) = opponent_pieces_bb.try_pop_lsb() {
                let attacks_from_piece = match piece_type {
                    Piece::Pawn => MOVE_TABLES.get_pawn_attacks(from_sq as usize, opponent),
                    Piece::Bishop => MOVE_TABLES
                        .get_bishop_attacks_generic(from_sq as usize, occupied_for_attacks),
                    Piece::Knight => MOVE_TABLES.knight_moves[from_sq as usize],
                    Piece::Rook => {
                        MOVE_TABLES.get_rook_attacks_generic(from_sq as usize, occupied_for_attacks)
                    }
                    Piece::Queen => {
                        MOVE_TABLES
                            .get_bishop_attacks_generic(from_sq as usize, occupied_for_attacks)
                            | MOVE_TABLES
                                .get_rook_attacks_generic(from_sq as usize, occupied_for_attacks)
                    }
                    _ => BitBoard(0),
                };

                if (attacks_from_piece & king_zone_mask).any() {
                    total_attack_units += match piece_type {
                        Piece::Pawn => 1,
                        Piece::Knight => 2,
                        Piece::Bishop => 2,
                        Piece::Rook => 3,
                        Piece::Queen => 5,
                        _ => 0,
                    };
                }
            }
        }

        total_attack_units
    }
}

impl Evaluator for KingSafetyEvaluator {
    fn evaluate(&self, board: &Board) -> Score {
        let mut white_score = 0;
        let mut black_score = 0;

        let white_king_sq = board.positions.get_piece_bb(Side::White, Piece::King).lsb();
        let black_king_sq = board.positions.get_piece_bb(Side::Black, Piece::King).lsb();

        if let Some(king_sq) = white_king_sq {
            white_score += self.evaluate_castling(board.castling_rights, Side::White);
            white_score += self.evaluate_pawn_shield(board, Side::White, king_sq as usize);
            white_score += self.evaluate_open_files(board, Side::White, king_sq as usize);
            let attack_units = self.calculate_attack_units(board, Side::White, king_sq as usize);
            white_score -= SAFETY_TABLE[attack_units.min(99) as usize];
        }

        if let Some(king_sq) = black_king_sq {
            black_score += self.evaluate_castling(board.castling_rights, Side::Black);
            black_score += self.evaluate_pawn_shield(board, Side::Black, king_sq as usize);
            black_score += self.evaluate_open_files(board, Side::Black, king_sq as usize);
            let attack_units = self.calculate_attack_units(board, Side::Black, king_sq as usize);
            black_score -= SAFETY_TABLE[attack_units.min(99) as usize];
        }

        let mg_score = white_score - black_score;
        let eg_score = mg_score / 4; // NOTE: KingSafety is much less relevant in endgame

        let score = Score::new(mg_score, eg_score);

        if board.stm == Side::White {
            score
        } else {
            -score
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn clone_box(&self) -> Box<dyn Evaluator> {
        Box::new(self.clone())
    }
}
