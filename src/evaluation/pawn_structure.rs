use crate::prelude::*;

#[derive(Debug)]
pub struct PawnStructureEvaluator {
    name: String,
    isolated_penalty: i32,
    doubled_penalty: i32,
    passed_bonus: i32,
}

impl Default for PawnStructureEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl PawnStructureEvaluator {
    pub fn new() -> Self {
        Self {
            name: "PawnStructure".to_string(),
            isolated_penalty: -15,
            doubled_penalty: -10,
            passed_bonus: 20,
        }
    }
}

impl Evaluator for PawnStructureEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        let mut score = 0;
        for &side in &[Side::White, Side::Black] {
            let pawns = board.positions.get_piece_bb(side, Piece::Pawn);
            let mut file_counts = [0u8; 8];
            for sq in pawns.iter_bits() {
                let file = sq % 8;
                file_counts[file] += 1;
            }
            for sq in pawns.iter_bits() {
                let file = sq % 8;
                // Isolated pawn: no friendly pawns on adjacent files
                let mut isolated = true;
                for adj in [file.wrapping_sub(1), file + 1]
                    .iter()
                    .cloned()
                    .filter(|&f| f < 8)
                {
                    if file_counts[adj] > 0 {
                        isolated = false;
                        break;
                    }
                }
                if isolated {
                    score += if side == Side::White {
                        self.isolated_penalty
                    } else {
                        -self.isolated_penalty
                    };
                }
                // Doubled pawn: more than one pawn on the file
                if file_counts[file] > 1 {
                    score += if side == Side::White {
                        self.doubled_penalty
                    } else {
                        -self.doubled_penalty
                    };
                }
                // Passed pawn: no enemy pawns on same or adjacent files ahead
                let enemy = if side == Side::White {
                    Side::Black
                } else {
                    Side::White
                };
                let enemy_pawns = board.positions.get_piece_bb(enemy, Piece::Pawn);
                let rank = sq / 8;
                let mut is_passed = true;
                for adj in [file.wrapping_sub(1), file, file + 1]
                    .iter()
                    .cloned()
                    .filter(|&f| f < 8)
                {
                    for enemy_sq in enemy_pawns.iter_bits() {
                        let enemy_file = enemy_sq % 8;
                        let enemy_rank = enemy_sq / 8;
                        if enemy_file == adj
                            && ((side == Side::White && enemy_rank > rank)
                                || (side == Side::Black && enemy_rank < rank))
                        {
                            is_passed = false;
                            break;
                        }
                    }
                    if !is_passed {
                        break;
                    }
                }
                if is_passed {
                    score += if side == Side::White {
                        self.passed_bonus
                    } else {
                        -self.passed_bonus
                    };
                }
            }
        }
        score
    }

    fn name(&self) -> &str {
        &self.name
    }
}
