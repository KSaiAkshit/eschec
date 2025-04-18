use super::Evaluator;
use crate::board::Board;

#[derive(Debug)]
pub struct MobilityEvaluator {
    name: String,
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
        }
    }
}

impl Evaluator for MobilityEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        // Generate all legal moves for current side
        let current_moves = match board.generate_legal_moves() {
            Ok(moves) => moves.len() as i32,
            Err(_) => 0,
        };

        // Create a temporary board with opponent to move
        let mut temp_board = *board;
        temp_board.stm = temp_board.stm.flip();

        // Generate all legal moves for opponent
        let opponent_moves = match temp_board.generate_legal_moves() {
            Ok(moves) => moves.len() as i32,
            Err(_) => 0,
        };

        // Return move difference (positive is good for white)
        current_moves - opponent_moves
    }

    fn name(&self) -> &str {
        &self.name
    }
}
