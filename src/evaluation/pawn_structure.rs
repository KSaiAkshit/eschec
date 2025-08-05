use crate::prelude::*;

#[derive(Debug)]
pub struct PawnStructureEvaluator {
    name: String,
    isolated_penalty: i32,
    doubled_penalty: i32,
    backward_penalty: i32,
    passed_bonus: i32,
    connected_bonus: i32,
    shield_bonus: i32,
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
            backward_penalty: -8,
            passed_bonus: 20,
            connected_bonus: 5,
            shield_bonus: 3,
        }
    }
    fn evaluate_side(&self, board: &Board, side: Side) -> i32 {
        let friendly_pawns = board.positions.get_piece_bb(side, Piece::Pawn);
        let opponent = side.flip();
        let opponent_pawns = board.positions.get_piece_bb(opponent, Piece::Pawn);
        let occupied = board.positions.get_occupied_bb();
        let mut file_counts = [0u8; 8];
        for sq in friendly_pawns.iter_bits() {
            let file = sq % 8;
            file_counts[file] += 1;
        }
        let mut score = 0;

        for sq in friendly_pawns.iter_bits() {
            let file = sq % 8;
            let rank = sq / 8;

            // Isolated pawn: no friendly pawns on adjacent files
            let isolated = (file == 0 || file_counts[file - 1] == 0)
                && (file == 7 || file_counts[file + 1] == 0);
            if isolated {
                score += self.isolated_penalty;
            }

            // Doubled pawn: more than one pawn on the file
            if file_counts[file] > 1 {
                score += self.doubled_penalty;
            }

            // Connected pawn: friendly pawn on adjacent file and rank
            let mut connected = false;
            for adj_file in [file.wrapping_sub(1), file + 1]
                .iter()
                .cloned()
                .filter(|&f| f < 8)
            {
                for adj_rank in [rank.wrapping_sub(1), rank + 1]
                    .iter()
                    .cloned()
                    .filter(|&r| r < 8)
                {
                    let adj_sq = adj_rank * 8 + adj_file;
                    if friendly_pawns.contains_square(adj_sq) {
                        connected = true;
                        break;
                    }
                }
                if connected {
                    break;
                }
            }
            if connected {
                score += self.connected_bonus;
            }

            // Passed pawn: no enemy pawns on same/adjacent files ahead
            let mut is_passed = true;
            for adj_file in [file.wrapping_sub(1), file, file + 1]
                .iter()
                .cloned()
                .filter(|&f| f < 8)
            {
                for opp in opponent_pawns.iter_bits() {
                    let opp_file = opp % 8;
                    let opp_rank = opp / 8;
                    let ahead = match side {
                        Side::White => opp_rank > rank,
                        Side::Black => opp_rank < rank,
                    };
                    if opp_file == adj_file && ahead {
                        is_passed = false;
                        break;
                    }
                }
                if !is_passed {
                    break;
                }
            }

            if is_passed {
                score += self.passed_bonus;
            }

            // Backward pawn: blocked in fron, no friendly pawn behind on adjacent files
            let forward = if side == Side::White { 1 } else { -1 };
            let next_rank = (rank as i8 + forward) as usize;
            if next_rank < 8 {
                let front_sq = next_rank * 8 + file;
                let is_blocked = occupied.contains_square(front_sq);
                let mut has_support = false;
                for adj_file in [file.wrapping_sub(1), file + 1]
                    .iter()
                    .cloned()
                    .filter(|&f| f < 8)
                {
                    let support_rank = (rank as i8 - forward) as usize;
                    if support_rank < 8 {
                        let support_sq = support_rank * 8 + adj_file;
                        if friendly_pawns.contains_square(support_sq) {
                            has_support = true;
                            break;
                        }
                    }
                }
                if is_blocked && !has_support {
                    score += self.backward_penalty;
                }
            }
        }

        // Pawn shield for king
        let king_sq = board.positions.get_piece_bb(side, Piece::King).lsb();
        if let Some(ksq) = king_sq {
            let kfile = ksq as usize % 8;
            let krank = ksq as usize / 8;
            let shield_rank = match side {
                Side::White => krank + 1,
                Side::Black => krank - 1,
            };
            if shield_rank < 8 {
                for df in [-1, 0, 1] {
                    let f = kfile as i8 + df;
                    if (0..8).contains(&f) {
                        let sq = shield_rank * 8 + f as usize;
                        if friendly_pawns.contains_square(sq) {
                            score += self.shield_bonus;
                        }
                    }
                }
            }
        }
        score
    }
}

impl Evaluator for PawnStructureEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        let white_score = self.evaluate_side(board, Side::White);
        let black_score = self.evaluate_side(board, Side::Black);
        let score = white_score - black_score;
        if board.stm == Side::White {
            score
        } else {
            -score
        }
    }
    fn name(&self) -> &str {
        &self.name
    }
}
