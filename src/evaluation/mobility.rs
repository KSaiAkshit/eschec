use std::collections::HashMap;

use super::{Evaluator, Piece};
use crate::board::Board;

#[derive(Debug)]
pub struct MobilityEvaluator {
    name: String,
    mobility_weights: HashMap<Piece, i32>,
}

impl Default for MobilityEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl MobilityEvaluator {
    pub fn new() -> Self {
        let mut mobility_weights = HashMap::new();
        mobility_weights.insert(Piece::Pawn, 1);
        mobility_weights.insert(Piece::Knight, 3);
        mobility_weights.insert(Piece::Bishop, 3);
        mobility_weights.insert(Piece::Rook, 5);
        mobility_weights.insert(Piece::Queen, 9);
        mobility_weights.insert(Piece::King, 0);

        Self {
            name: "Mobility".to_string(),
            mobility_weights,
        }
    }

    fn calculate_mobility_score(&self, board: &Board) -> i32 {
        let legal_moves = board.generate_legal_moves();
        let mut score = 0;

        for m in legal_moves {
            if let Some(piece) = board.get_piece_at(m.from_sq())
                && let Some(weight) = self.mobility_weights.get(&piece)
            {
                score += weight;
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
