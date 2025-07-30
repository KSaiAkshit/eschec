use crate::prelude::*;
#[derive(Debug)]
pub struct KingSafetyEvaluator {
    name: String,
    castling_bonus: i32,
}

impl Default for KingSafetyEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl KingSafetyEvaluator {
    pub fn new() -> Self {
        Self {
            name: "KingSafety".to_string(),
            castling_bonus: 25,
        }
    }
    fn evaluate_castling(&self, rights: CastlingRights, side: Side) -> i32 {
        let mut score = 0;

        if rights.can_castle(side, true) {
            score += self.castling_bonus;
        }

        if rights.can_castle(side, false) {
            score += self.castling_bonus;
        }

        score
    }
}

impl Evaluator for KingSafetyEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        let castling_rights = board.castling_rights;

        let score = self.evaluate_castling(castling_rights, Side::White)
            - self.evaluate_castling(castling_rights, Side::Black);

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
