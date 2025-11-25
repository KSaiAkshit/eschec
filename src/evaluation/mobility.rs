use std::sync::RwLock;

use crate::{prelude::*, tuning::params::TunableParams};

const MOBILITY_WEIGHTS: [Score; NUM_PIECES] = [
    Score::new(1, 2),
    Score::new(4, 4),
    Score::new(4, 4),
    Score::new(3, 6),
    Score::new(2, 4),
    Score::new(0, 0),
];

#[derive(Debug)]
pub struct MobilityEvaluator {
    name: String,
    mobility_weights: [Score; NUM_PIECES],
    move_buffer: RwLock<MoveBuffer>,
}

impl Default for MobilityEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl MobilityEvaluator {
    pub fn new() -> Self {
        Self {
            name: "Mobility".to_owned(),
            mobility_weights: MOBILITY_WEIGHTS,
            move_buffer: RwLock::new(MoveBuffer::new()),
        }
    }

    pub fn with_params(params: &TunableParams) -> Self {
        let mut mob = [Score::default(); NUM_PIECES];
        mob.iter_mut().enumerate().for_each(|(i, s)| {
            *s = params.mobility[i];
        });

        Self {
            name: "Mobility".to_owned(),
            mobility_weights: mob,
            move_buffer: RwLock::new(MoveBuffer::new()),
        }
    }

    fn calculate_mobility_score(&self, board: &Board) -> Score {
        let mut buffer = self
            .move_buffer
            .write()
            .expect("Failed to acquire write lock on move_buffer");
        // NOTE: Using pseudo legal moves here so that its faster
        // and is good enough for this.
        board.generate_pseudo_legal_moves(&mut buffer);
        let mut score = Score::default();

        for m in &*buffer {
            if let Some(piece) = board.get_piece_at(m.from_sq()) {
                score += self.mobility_weights[piece.index()];
            }
        }
        buffer.clear();
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
        let score = current_player_mobility - opponent_mobility;

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
        let cloned_eval = Self {
            name: self.name.clone(),
            mobility_weights: self.mobility_weights,
            move_buffer: RwLock::new(
                *self
                    .move_buffer
                    .read()
                    .expect("Failed to acquire read-lock on move_buffer"),
            ),
        };
        Box::new(cloned_eval)
    }
}
