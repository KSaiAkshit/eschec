use crate::prelude::Score;

// Material
pub const BISHOP_PAIR_BONUS_MG: usize = 0;
pub const BISHOP_PAIR_BONUS_EG: usize = 1;
// King Safety
pub const CASTLING_BONUS: usize = 2;
pub const PAWN_SHIELD_FULL: usize = 3;
pub const PAWN_SHIELD_PARTIAL: usize = 4;
pub const OPEN_FILE_PENALTY: usize = 5;
// Pawn Structure
pub const ISOLATED_PENALTY_MG: usize = 6;
pub const ISOLATED_PENALTY_EG: usize = 7;
pub const DOUBLED_PENALTY_MG: usize = 8;
pub const DOUBLED_PENALTY_EG: usize = 9;
pub const BACKWARD_PENALTY_MG: usize = 10;
pub const BACKWARD_PENALTY_EG: usize = 11;
pub const CONNECTED_BONUS_MG: usize = 12;
pub const CONNECTED_BONUS_EG: usize = 13;
// Passed pawn scores = 16 params
pub const PASSED_PAWN_SCORES_START: usize = 14;
pub const PASSED_PAWN_SCORES_END: usize = PASSED_PAWN_SCORES_START + 16 - 1;
// Position
pub const ROOK_OPEN_FILE_BONUS_MG: usize = 30;
pub const ROOK_OPEN_FILE_BONUS_EG: usize = 31;
pub const ROOK_SEMI_FILE_BONUS_MG: usize = 32;
pub const ROOK_SEMI_FILE_BONUS_EG: usize = 33;
pub const KNIGHT_OUTPOST_BONUS_MG: usize = 34;
pub const KNIGHT_OUTPOST_BONUS_EG: usize = 35;
// Mobility
pub const MOBILITY_PAWN: usize = 36;
pub const MOBILITY_KNIGHT: usize = 37;
pub const MOBILITY_BISHOP: usize = 38;
pub const MOBILITY_ROOK: usize = 39;
pub const MOBILITY_QUEEN: usize = 40;

pub const NUM_TUNABLE_PARAMS: usize = 41;

pub struct TunableParams {
    // Material (2 params)
    pub bishop_pair_bonus: Score,
    // King Safety (4 params)
    pub castling_bonus: i32,
    pub pawn_shield_full: i32,
    pub pawn_shield_partial: i32,
    pub open_file_penalty: i32,
    // Pawn structure (24 params)
    pub isolated_penalty: Score,
    pub doubled_penalty: Score,
    pub backward_penalty: Score,
    pub passed_pawn_scores: [Score; 8],
    pub connected_bonus: Score,
    // Position (6 params)
    pub rook_open_file_bonus: Score,
    pub rook_semi_file_bonus: Score,
    pub knight_outpost_bonus: Score,
    // Mobility (5 params)
    pub mobility_pawn: i32,
    pub mobility_knight: i32,
    pub mobility_bishop: i32,
    pub mobility_rook: i32,
    pub mobility_queen: i32,
}

/// Params to start off with
impl Default for TunableParams {
    fn default() -> Self {
        let passed_pawn_scores = [
            Score { mg: 0, eg: 0 },
            Score { mg: 5, eg: 10 },
            Score { mg: 10, eg: 30 },
            Score { mg: 25, eg: 60 },
            Score { mg: 60, eg: 150 },
            Score { mg: 150, eg: 400 },
            Score { mg: 300, eg: 700 },
            Score { mg: 0, eg: 0 },
        ];
        Self {
            bishop_pair_bonus: Score::new(26, 40),
            castling_bonus: 10,
            pawn_shield_full: 30,
            pawn_shield_partial: 10,
            open_file_penalty: -40,
            isolated_penalty: Score::splat(-15),
            doubled_penalty: Score::splat(-10),
            backward_penalty: Score::splat(-8),
            passed_pawn_scores,
            connected_bonus: Score::splat(2),
            rook_open_file_bonus: Score::new(40, 20),
            rook_semi_file_bonus: Score::new(20, 10),
            knight_outpost_bonus: Score::new(30, 15),
            mobility_pawn: 1,
            mobility_knight: 3,
            mobility_bishop: 3,
            mobility_rook: 5,
            mobility_queen: 9,
        }
    }
}

impl TunableParams {
    pub fn to_vector(&self) -> Vec<f64> {
        let mut vec: Vec<f64> = vec![0.0; NUM_TUNABLE_PARAMS];
        vec[BISHOP_PAIR_BONUS_MG] = self.bishop_pair_bonus.mg as f64;
        vec[BISHOP_PAIR_BONUS_EG] = self.bishop_pair_bonus.eg as f64;
        vec[CASTLING_BONUS] = self.castling_bonus as f64;
        vec[PAWN_SHIELD_FULL] = self.pawn_shield_full as f64;
        vec[PAWN_SHIELD_PARTIAL] = self.pawn_shield_partial as f64;
        vec[OPEN_FILE_PENALTY] = self.open_file_penalty as f64;
        vec[ISOLATED_PENALTY_MG] = self.isolated_penalty.mg as f64;
        vec[ISOLATED_PENALTY_EG] = self.isolated_penalty.eg as f64;
        vec[DOUBLED_PENALTY_MG] = self.doubled_penalty.mg as f64;
        vec[DOUBLED_PENALTY_EG] = self.doubled_penalty.eg as f64;
        vec[BACKWARD_PENALTY_MG] = self.backward_penalty.mg as f64;
        vec[BACKWARD_PENALTY_EG] = self.backward_penalty.eg as f64;

        // Fill passed pawn scores here
        for i in 0..8 {
            // Each rank `i` has two values (mg, eg) that need to be placed
            // in the vector. The offset is `i * 2`.
            let base_idx = PASSED_PAWN_SCORES_START + i * 2;
            vec[base_idx] = self.passed_pawn_scores[i].mg as f64;
            vec[base_idx + 1] = self.passed_pawn_scores[i].eg as f64;
        }
        vec[CONNECTED_BONUS_MG] = self.connected_bonus.mg as f64;
        vec[CONNECTED_BONUS_EG] = self.connected_bonus.eg as f64;
        vec[ROOK_OPEN_FILE_BONUS_MG] = self.rook_open_file_bonus.mg as f64;
        vec[ROOK_OPEN_FILE_BONUS_EG] = self.rook_open_file_bonus.eg as f64;
        vec[ROOK_SEMI_FILE_BONUS_MG] = self.rook_semi_file_bonus.mg as f64;
        vec[ROOK_SEMI_FILE_BONUS_EG] = self.rook_semi_file_bonus.eg as f64;
        vec[KNIGHT_OUTPOST_BONUS_MG] = self.knight_outpost_bonus.mg as f64;
        vec[KNIGHT_OUTPOST_BONUS_EG] = self.knight_outpost_bonus.eg as f64;
        vec[MOBILITY_PAWN] = self.mobility_pawn as f64;
        vec[MOBILITY_KNIGHT] = self.mobility_knight as f64;
        vec[MOBILITY_BISHOP] = self.mobility_bishop as f64;
        vec[MOBILITY_ROOK] = self.mobility_rook as f64;
        vec[MOBILITY_QUEEN] = self.mobility_queen as f64;

        vec
    }

    pub fn from_vector(vec: &[f64]) -> Self {
        let mut passed_pawn_scores = [Score::default(); 8];
        for (i, score) in passed_pawn_scores.iter_mut().enumerate() {
            let base_idx = PASSED_PAWN_SCORES_START + i * 2;
            *score = Score::new(vec[base_idx] as i32, vec[base_idx + 1] as i32);
        }

        Self {
            // Material
            bishop_pair_bonus: Score::new(
                vec[BISHOP_PAIR_BONUS_MG] as i32,
                vec[BISHOP_PAIR_BONUS_EG] as i32,
            ),

            // King Safety
            castling_bonus: vec[CASTLING_BONUS] as i32,
            pawn_shield_full: vec[PAWN_SHIELD_FULL] as i32,
            pawn_shield_partial: vec[PAWN_SHIELD_PARTIAL] as i32,
            open_file_penalty: vec[OPEN_FILE_PENALTY] as i32,

            // Pawn Structure
            isolated_penalty: Score::new(
                vec[ISOLATED_PENALTY_MG] as i32,
                vec[ISOLATED_PENALTY_EG] as i32,
            ),
            doubled_penalty: Score::new(
                vec[DOUBLED_PENALTY_MG] as i32,
                vec[DOUBLED_PENALTY_EG] as i32,
            ),
            backward_penalty: Score::new(
                vec[BACKWARD_PENALTY_MG] as i32,
                vec[BACKWARD_PENALTY_EG] as i32,
            ),
            connected_bonus: Score::new(
                vec[CONNECTED_BONUS_MG] as i32,
                vec[CONNECTED_BONUS_EG] as i32,
            ),

            // Assign the reconstructed array
            passed_pawn_scores,

            // Position
            rook_open_file_bonus: Score::new(
                vec[ROOK_OPEN_FILE_BONUS_MG] as i32,
                vec[ROOK_OPEN_FILE_BONUS_EG] as i32,
            ),
            rook_semi_file_bonus: Score::new(
                vec[ROOK_SEMI_FILE_BONUS_MG] as i32,
                vec[ROOK_SEMI_FILE_BONUS_EG] as i32,
            ),
            knight_outpost_bonus: Score::new(
                vec[KNIGHT_OUTPOST_BONUS_MG] as i32,
                vec[KNIGHT_OUTPOST_BONUS_EG] as i32,
            ),

            // Mobility
            mobility_pawn: vec[MOBILITY_PAWN] as i32,
            mobility_knight: vec[MOBILITY_KNIGHT] as i32,
            mobility_bishop: vec[MOBILITY_BISHOP] as i32,
            mobility_rook: vec[MOBILITY_ROOK] as i32,
            mobility_queen: vec[MOBILITY_QUEEN] as i32,
        }
    }
}
