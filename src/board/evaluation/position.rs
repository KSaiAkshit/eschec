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
        let pawn_tables = [
            0, 0, 0, 0, 0, 0, 0, 0, 50, 50, 50, 50, 50, 50, 50, 50, 10, 10, 20, 30, 30, 20, 10, 10,
            5, 5, 10, 25, 25, 10, 5, 5, 0, 0, 0, 20, 20, 0, 0, 0, 5, -5, -10, 0, 0, -10, -5, 5, 5,
            10, 10, -20, -20, 10, 10, 5, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let default_table = [0; 64];
        Self {
            name: "Position".to_string(),
            piece_square_tables: [
                pawn_tables,
                default_table,
                default_table,
                default_table,
                default_table,
                default_table,
            ],
        }
    }
}

impl Evalutor for PositionEvaluator {
    fn evaluate(&self, board: &mut Board) -> i32 {
        todo!()
    }

    fn name(&self) -> &str {
        &self.name
    }
}
