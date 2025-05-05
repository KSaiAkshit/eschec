use super::*;

#[derive(Debug)]
pub struct PositionEvaluator {
    name: String,
    piece_square_tables: [[i32; 64]; 6],
}

impl Default for PositionEvaluator {
    fn default() -> Self {
        Self {
            name: "Position".to_string(),
            piece_square_tables: [[0; 64]; 6],
        }
    }
}

impl PositionEvaluator {
    pub fn new() -> Self {
        // Just advance
        #[rustfmt::skip]
        let pawn_tables = [
            0,  0,  0,  0,  0,  0,  0,  0,
            50, 50, 50, 50, 50, 50, 50, 50,
            10, 10, 20, 30, 30, 20, 10, 10,
            5,  5, 10, 25, 25, 10,  5,  5,
            0,  0,  0, 20, 20,  0,  0,  0,
            5, -5,-10,  0,  0,-10, -5,  5,
            5, 10, 10,-20,-20, 10, 10,  5,
            0,  0,  0,  0,  0,  0,  0,  0
        ];

        // Go towards the center
        #[rustfmt::skip]
        let knight_table = [
            -50,-40,-30,-30,-30,-30,-40,-50,
            -40,-20,  0,  0,  0,  0,-20,-40,
            -30,  0, 10, 15, 15, 10,  0,-30,
            -30,  5, 15, 20, 20, 15,  5,-30,
            -30,  0, 15, 20, 20, 15,  0,-30,
            -30,  5, 10, 15, 15, 10,  5,-30,
            -40,-20,  0,  5,  5,  0,-20,-40,
            -50,-40,-30,-30,-30,-30,-40,-50,
        ];

        // Avoid corners and borderes
        #[rustfmt::skip]
        let bishop_table = [
            -20,-10,-10,-10,-10,-10,-10,-20,
            -10,  0,  0,  0,  0,  0,  0,-10,
            -10,  0,  5, 10, 10,  5,  0,-10,
            -10,  5,  5, 10, 10,  5,  5,-10,
            -10,  0, 10, 10, 10, 10,  0,-10,
            -10, 10, 10, 10, 10, 10, 10,-10,
            -10,  5,  0,  0,  0,  0,  5,-10,
            -20,-10,-10,-10,-10,-10,-10,-20,
        ];

        #[rustfmt::skip]
        let rook_table = [
            0,  0,  0,  0,  0,  0,  0,  0,
            5, 10, 10, 10, 10, 10, 10,  5,
            -5,  0,  0,  0,  0,  0,  0, -5,
            -5,  0,  0,  0,  0,  0,  0, -5,
            -5,  0,  0,  0,  0,  0,  0, -5,
            -5,  0,  0,  0,  0,  0,  0, -5,
            -5,  0,  0,  0,  0,  0,  0, -5,
             0,  0,  0,  5,  5,  0,  0,  0
        ];

        // Mostly central squares good
        #[rustfmt::skip]
        let queen_table = [
            -20,-10,-10, -5, -5,-10,-10,-20,
            -10,  0,  0,  0,  0,  0,  0,-10,
            -10,  0,  5,  5,  5,  5,  0,-10,
            -5,  0,  5,  5,  5,  5,  0, -5,
             0,  0,  5,  5,  5,  5,  0, -5,
            -10,  5,  5,  5,  5,  5,  0,-10,
            -10,  0,  5,  0,  0,  0,  0,-10,
            -20,-10,-10, -5, -5,-10,-10,-20
        ];

        // King midgame table
        #[rustfmt::skip]
        let king_table = [
            -30,-40,-40,-50,-50,-40,-40,-30,
            -30,-40,-40,-50,-50,-40,-40,-30,
            -30,-40,-40,-50,-50,-40,-40,-30,
            -30,-40,-40,-50,-50,-40,-40,-30,
            -20,-30,-30,-40,-40,-30,-30,-20,
            -10,-20,-20,-20,-20,-20,-20,-10,
             20, 20,  0,  0,  0,  0, 20, 20,
             20, 30, 10,  0,  0, 10, 30, 20
        ];

        // TODO: Add king_ending and king_midgame table
        Self {
            name: "Position".to_string(),
            piece_square_tables: [
                pawn_tables,
                knight_table,
                bishop_table,
                rook_table,
                queen_table,
                king_table,
            ],
        }
    }
}

impl Evaluator for PositionEvaluator {
    fn evaluate(&self, board: &Board) -> i32 {
        // Source: https://www.chessprogramming.org/Simplified_Evaluation_Function
        // Add rewards or penlaties for pieces at different squares.
        // 3 rules. Also called PST (Piece Square table)
        // 1.) Bonuses for good squares
        // 2.) Penalties for bad squares
        // 3.) Neutral Value of 0 for other squares
        let mut score = 0;

        for piece in Piece::all_pieces() {
            let piece_idx = piece.index();
            let piece_table = &self.piece_square_tables[piece_idx];
            for idx in &board
                .positions
                .get_piece_bb(&Side::White, &piece)
                .get_set_bits()
            {
                score += piece_table[*idx];
            }
            for idx in &board
                .positions
                .get_piece_bb(&Side::Black, &piece)
                .get_set_bits()
            {
                let mirrored_idx = 63 - idx;
                score -= piece_table[mirrored_idx];
            }
        }
        score
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_pawn() {
        let mut board = Board::from_fen("8/8/8/8/8/8/PPPPPPPP/8 w KQkq - 0 1");
        println!("{}", board);
        board.stm = board.stm.flip();
        let eval = PositionEvaluator::new();
        let score = eval.evaluate(&board);
        assert_eq!(score, 400);
    }
    #[test]
    fn test_default_board() {
        let board = Board::new();
        let eval = PositionEvaluator::new();
        let score = eval.evaluate(&board);
        assert_eq!(score, 0);
    }
}
