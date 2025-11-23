use crate::{prelude::*, tuning::params::TunableParams};

#[derive(Debug, Clone)]
pub struct PawnStructureEvaluator {
    name: String,
    isolated_penalty: Score,
    doubled_penalty: Score,
    backward_penalty: Score,
    passed_pawn_scores: [Score; 8],
    connected_bonus: Score,
}

impl Default for PawnStructureEvaluator {
    fn default() -> Self {
        Self {
            name: "PawnStructure".to_owned(),
            isolated_penalty: Score::default(),
            doubled_penalty: Score::default(),
            backward_penalty: Score::default(),
            passed_pawn_scores: [Score::default(); 8],
            connected_bonus: Score::default(),
        }
    }
}

impl PawnStructureEvaluator {
    pub fn new() -> Self {
        let scores = [
            Score { mg: 0, eg: 0 },     // Rank 1 (Impossible)
            Score { mg: 5, eg: 10 },    // Rank 2
            Score { mg: 10, eg: 30 },   // Rank 3
            Score { mg: 25, eg: 60 },   // Rank 4
            Score { mg: 60, eg: 150 },  // Rank 5 (Must be defended)
            Score { mg: 150, eg: 400 }, // Rank 6 (Worth more than a minor piece)
            Score { mg: 300, eg: 700 }, // Rank 7 (Almost a Queen)
            Score { mg: 0, eg: 0 },     // Rank 8 (Impossible/Promoted)
        ];
        Self {
            name: "PawnStructure".to_owned(),
            isolated_penalty: Score::splat(-15),
            doubled_penalty: Score::splat(-10),
            backward_penalty: Score::splat(-8),
            passed_pawn_scores: scores,
            connected_bonus: Score::splat(2),
        }
    }

    pub fn with_params(params: &TunableParams) -> Self {
        Self {
            name: "PawnStructure".to_owned(),
            isolated_penalty: params.isolated_penalty,
            doubled_penalty: params.doubled_penalty,
            backward_penalty: params.backward_penalty,
            passed_pawn_scores: params.passed_pawn_scores,
            connected_bonus: params.connected_bonus,
        }
    }
    fn evaluate_side(&self, board: &Board, side: Side) -> Score {
        let friendly_pawns = board.positions.get_piece_bb(side, Piece::Pawn);
        let opponent = side.flip();
        let opponent_pawns = board.positions.get_piece_bb(opponent, Piece::Pawn);
        let occupied = board.positions.get_occupied_bb();

        let mut score = Score::default();

        for sq_idx in friendly_pawns.iter_bits() {
            let relative_rank = if side == Side::White {
                sq_idx / 8
            } else {
                7 - (sq_idx / 8)
            };
            let file = sq_idx % 8;

            if (friendly_pawns & &PAWN_TABLES.pawn_adjacent_files_masks[file]).is_empty() {
                score += self.isolated_penalty;
            }

            if (friendly_pawns.0 & FILE_MASKS[file]).count_ones() > 1 {
                score += self.doubled_penalty;
            }

            // If there are no enemy pawns in front of our pawn (file - 1, file, file + 1),
            // then this is a passed pawn.
            if (opponent_pawns & &PAWN_TABLES.passed_pawn_blocking_masks[side.index()][sq_idx])
                .is_empty()
            {
                score += self.passed_pawn_scores[relative_rank];
            }

            let is_blocked_in_front =
                (occupied & PAWN_TABLES.pawn_front_square_masks[side.index()][sq_idx]).any();
            let has_backward_support = (friendly_pawns
                & &PAWN_TABLES.pawn_backward_support_masks[side.index()][sq_idx])
                .any();
            if is_blocked_in_front && !has_backward_support {
                score += self.backward_penalty;
            }

            let connected_neighbors = friendly_pawns & &PAWN_TABLES.connected_pawn_masks[sq_idx];
            score += self.connected_bonus * connected_neighbors.pop_count() as i32;
        }
        score
    }
}

impl Evaluator for PawnStructureEvaluator {
    fn evaluate(&self, board: &Board) -> Score {
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
    fn clone_box(&self) -> Box<dyn Evaluator> {
        Box::new(self.clone())
    }
}
