use super::Evaluator;
use crate::{board::Board, consts::NUM_PIECES};

const MOBILITY_WEIGHTS: [i32; NUM_PIECES] = [1, 3, 3, 5, 9, 0];
#[derive(Debug)]
pub struct MobilityEvaluator {
    name: String,
    mobility_weights: [i32; NUM_PIECES],
}

impl Default for MobilityEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl MobilityEvaluator {
    pub fn new() -> Self {
        Self {
            name: "Mobility".to_string(),
            mobility_weights: MOBILITY_WEIGHTS,
        }
    }

    fn calculate_mobility_score(&self, board: &Board) -> i32 {
        // NOTE: Use pseudo legal moves here;
        let legal_moves = board.generate_legal_moves();
        let mut score = 0;

        for m in legal_moves {
            if let Some(piece) = board.get_piece_at(m.from_sq()) {
                score += self.mobility_weights[piece.index()];
            }
        }
        score
    }
}

impl Evaluator for MobilityEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        let current_player_mobility = self.calculate_mobility_score(board);

        // To calculate for the opponent, we need a board state from their perspective.
        let mut opponent_board = *board;
        opponent_board.stm = opponent_board.stm.flip();

        let opponent_mobility = self.calculate_mobility_score(&opponent_board);

        // The final score is the difference in mobility.
        current_player_mobility - opponent_mobility
    }

    fn name(&self) -> &str {
        &self.name
    }
}
