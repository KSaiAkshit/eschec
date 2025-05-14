use std::collections::HashMap;

use super::{Evaluator, Piece, Square};
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
        mobility_weights.insert(Piece::Knight, 4);
        mobility_weights.insert(Piece::Bishop, 4);
        mobility_weights.insert(Piece::Rook, 2);
        mobility_weights.insert(Piece::Queen, 2);
        mobility_weights.insert(Piece::King, 0);

        Self {
            name: "Mobility".to_string(),
            mobility_weights,
        }
    }

    fn evaluate_move_by_piece(&self, legal_moves: &HashMap<Piece, Vec<(Square, Square)>>) -> i32 {
        let mut score = 0;

        for k in legal_moves.keys() {
            let entries = legal_moves.get(k);
            let len = if let Some(entries) = entries {
                entries.len() as i32
            } else {
                0
            };

            if let Some(&weight) = self.mobility_weights.get(k) {
                score += weight * len;
            }
        }

        score
    }
}

impl Evaluator for MobilityEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        let current_moves = match board.generate_piecewise_legal_moves() {
            Ok(moves) => self.evaluate_move_by_piece(&moves),
            Err(_) => 0,
        };

        let mut temp_board = *board;
        temp_board.stm = temp_board.stm.flip();

        let opponent_moves = match temp_board.generate_piecewise_legal_moves() {
            Ok(moves) => self.evaluate_move_by_piece(&moves),
            Err(_) => 0,
        };

        current_moves - opponent_moves
    }

    fn name(&self) -> &str {
        &self.name
    }
}
