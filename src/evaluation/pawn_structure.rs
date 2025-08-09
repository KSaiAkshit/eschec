use crate::prelude::*;

#[derive(Debug)]
pub struct PawnStructureEvaluator {
    name: String,
    isolated_penalty: i32,
    doubled_penalty: i32,
    backward_penalty: i32,
    passed_bonus: i32,
    connected_bonus: i32,
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
            passed_bonus: 10,
            connected_bonus: 2,
        }
    }
    fn evaluate_side(&self, board: &Board, side: Side) -> i32 {
        let friendly_pawns = board.positions.get_piece_bb(side, Piece::Pawn);
        let opponent = side.flip();
        let opponent_pawns = board.positions.get_piece_bb(opponent, Piece::Pawn);
        let occupied = board.positions.get_occupied_bb();

        let mut score = 0;

        for sq_idx in friendly_pawns.iter_bits() {
            let _rank = sq_idx / 8;
            let file = sq_idx % 8;

            if (friendly_pawns & &PAWN_TABLES.pawn_adjacent_files_masks[file]).is_empty() {
                score += self.isolated_penalty;
            }

            if (friendly_pawns.0 & FILE_MASKS[file]).count_ones() > 1 {
                score += self.doubled_penalty;
            }

            if (opponent_pawns & &PAWN_TABLES.passed_pawn_blocking_masks[side.index()][sq_idx])
                .is_empty()
            {
                score += self.passed_bonus;
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
            score += connected_neighbors.pop_count() as i32 * self.connected_bonus;
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
