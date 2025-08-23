use crate::prelude::*;

const MOBILITY_WEIGHTS: [i32; NUM_PIECES] = [1, 3, 3, 5, 9, 0];
#[derive(Debug, Clone)]
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
        let mut buffer = MoveBuffer::new();
        // NOTE: Using pseudo legal moves here so that its faster
        // and is good enough for this.
        board.generate_pseudo_legal_moves(&mut buffer);
        let mut score = 0;

        for m in &buffer {
            if let Some(piece) = board.get_piece_at(m.from_sq()) {
                score += self.mobility_weights[piece.index()];
            }
        }
        score
    }
}

impl Evaluator for MobilityEvaluator {
    fn evaluate(&self, board: &Board) -> Score {
        let current_player_mobility = self.calculate_mobility_score(board);

        // To calculate for the opponent, we need a board state from their perspective.
        let mut opponent_board = *board;
        opponent_board.stm = opponent_board.stm.flip();

        let opponent_mobility = self.calculate_mobility_score(&opponent_board);

        // The final score is the difference in mobility.
        // TODO: How to taper here?
        let score = Score::splat(current_player_mobility - opponent_mobility);

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
