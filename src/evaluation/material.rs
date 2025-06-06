use super::*;

#[derive(Debug)]
pub struct MaterialEvaluator {
    name: String,
}

impl Default for MaterialEvaluator {
    fn default() -> Self {
        Self {
            name: "Material".to_string(),
        }
    }
}

impl MaterialEvaluator {
    pub fn new() -> Self {
        Self {
            name: "Material".to_string(),
        }
    }
}

impl Evaluator for MaterialEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
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

        let white_material = board
            .positions
            .get_colored_pieces(Side::White)
            .iter()
            .zip(Piece::colored_pieces(Side::White))
            .map(|(piece_bb, piece)| (piece_bb.0.count_ones() * piece.value()) as i32)
            .sum::<i32>();

        let black_material = board
            .positions
            .get_colored_pieces(Side::Black)
            .iter()
            .zip(Piece::colored_pieces(Side::Black))
            .map(|(piece_bb, piece)| (piece_bb.0.count_ones() * piece.value()) as i32)
            .sum::<i32>();

        let score = white_material - black_material;
        
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
}
