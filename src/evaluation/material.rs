use crate::{prelude::*, tuning::params::TunableParams};

#[derive(Debug, Clone)]
pub struct MaterialEvaluator {
    name: String,
    bishop_pair_bonus: Score,
}

impl Default for MaterialEvaluator {
    fn default() -> Self {
        Self {
            name: "Material".to_owned(),
            bishop_pair_bonus: Score::default(),
        }
    }
}

impl MaterialEvaluator {
    pub fn new() -> Self {
        Self {
            name: "Material".to_owned(),
            bishop_pair_bonus: Score::new(26, 40),
        }
    }
    pub fn with_params(params: &TunableParams) -> Self {
        Self {
            name: "Material".to_owned(),
            bishop_pair_bonus: params.bishop_pair_bonus,
        }
    }
    fn evaluate_bishop_pair(&self, board: &Board) -> Score {
        let mut score = Score::default();

        if board
            .positions
            .get_piece_bb(Side::White, Piece::Bishop)
            .pop_count()
            >= 2
        {
            score += self.bishop_pair_bonus;
        }

        if board
            .positions
            .get_piece_bb(Side::Black, Piece::Bishop)
            .pop_count()
            >= 2
        {
            score -= self.bishop_pair_bonus
        }
        score
    }
}

impl Evaluator for MaterialEvaluator {
    fn evaluate(&self, board: &Board) -> Score {
        // Source: https://www.chessprogramming.org/Simplified_Evaluation_Function
        // 4 rules of thumb
        // 1.) Avoid expaching one minor piece for 3 pawns
        // 2.) Encourage engine to have the bishop pair
        // 3.) Avoid exchanging 2 minor pieces for a rook and a pawn
        // 4.) Stick to human chess experience
        //
        // Final Equations:
        // ```
        // B > N > 3P
        // B + N = R + 1.5P
        // Q + P = 2R
        // ```

        let mut score = board.material[Side::White.index()] - board.material[Side::Black.index()];
        score += self.evaluate_bishop_pair(board);

        // Convert to side-to-move perspective
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
