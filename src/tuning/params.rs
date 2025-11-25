use std::{fs, path::Path};

use crate::prelude::{NUM_PIECES, NUM_SQUARES, Piece, Score};
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

// LOGICAL INDICES (For Accumulator)
// These are indices into the 'features' array of the Trace.
// They represent the *Feature*, not the *Weight*.
// Since a feature (like "Bishop Pair") has both an MG and EG weight,
// the SPSA vector will be 2x the size of these indices.

// Material Values
pub const MATERIAL_PAWN: usize = 0;
pub const MATERIAL_KNIGHT: usize = 1;
pub const MATERIAL_BISHOP: usize = 2;
pub const MATERIAL_ROOK: usize = 3;
pub const MATERIAL_QUEEN: usize = 4;

// Material Terms
pub const BISHOP_PAIR_BONUS: usize = 5;

// King Safety
pub const CASTLING_BONUS: usize = 6;
pub const PAWN_SHIELD_FULL: usize = 7;
pub const PAWN_SHIELD_PARTIAL: usize = 8;
pub const OPEN_FILE_PENALTY: usize = 9;
pub const SEMI_OPEN_FILE_PENALTY: usize = 10;

// Pawn Structure
pub const ISOLATED_PENALTY: usize = 11;
pub const DOUBLED_PENALTY: usize = 12;
pub const BACKWARD_PENALTY: usize = 13;
pub const CONNECTED_BONUS: usize = 14;

// Passed Pawns (8 ranks)
pub const PASSED_PAWN_START: usize = 15; // 15..22

// Position Bonuses
pub const ROOK_OPEN_FILE_BONUS: usize = 23;
pub const ROOK_SEMI_FILE_BONUS: usize = 24;
pub const KNIGHT_OUTPOST_BONUS: usize = 25;

// PSTs (6 pieces * 64 squares = 384 params)
pub const PST_START: usize = 26;
pub const NUM_PST_PARAMS: usize = NUM_PIECES * NUM_SQUARES;

// Total number of LOGICAL features (for Trace)
// 26 + 384 = 410
pub const NUM_TRACE_FEATURES: usize = PST_START + NUM_PST_PARAMS;

// Mobility is handled separately in Trace (i16 array),
// but we need to include it in the SPSA vector.
pub const MOBILITY_PARAMS_COUNT: usize = 5;

// Total number of weights in the SPSA vector
// (Features * 2) + (Mobility * 2)
// We multiply by 2 because every feature has MG and EG.
pub const SPSA_VECTOR_SIZE: usize = (NUM_TRACE_FEATURES * 2) + (MOBILITY_PARAMS_COUNT * 2);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunableParams {
    // Index 0=Pawn, 1=Knight, 2=Bishop, 3=Rook, 4=Queen
    pub material: [Score; 5],
    // Material
    pub bishop_pair_bonus: Score,

    // King Safety
    pub castling_bonus: Score,
    pub pawn_shield_full: Score,
    pub pawn_shield_partial: Score,
    pub open_file_penalty: Score,
    pub semi_open_file_penalty: Score,

    // Pawn Structure
    pub isolated_penalty: Score,
    pub doubled_penalty: Score,
    pub backward_penalty: Score,
    pub passed_pawn_scores: [Score; 8],
    pub connected_bonus: Score,

    // Position
    pub rook_open_file_bonus: Score,
    pub rook_semi_file_bonus: Score,
    pub knight_outpost_bonus: Score,

    // Mobility
    pub mobility: [Score; 5], // Pawn, Knight, Bishop, Rook, Queen

    // PSTs
    #[serde(with = "BigArray")]
    pub psts: [Score; NUM_PST_PARAMS],
}

impl Default for TunableParams {
    fn default() -> Self {
        #[rustfmt::skip]
        let mg_pawn_table = [
            0,   0,   0,   0,   0,   0,  0,   0,
            98, 134,  61,  95,  68, 126, 34, -11,
            -6,   7,  26,  31,  65,  56, 25, -20,
            -14,  13,   6,  21,  23,  12, 17, -23,
            -27,  -2,  -5,  12,  17,   6, 10, -25,
            -26,  -4,  -4, -10,   3,   3, 33, -12,
            -35,  -1, -20, -23, -15,  24, 38, -22,
            0,   0,   0,   0,   0,   0,  0,   0
        ];
        #[rustfmt::skip]
        let eg_pawn_table = [
            0,   0,   0,   0,   0,   0,   0,   0,
            178, 173, 158, 134, 147, 132, 165, 187,
            94, 100,  85,  67,  56,  53,  82,  84,
            32,  24,  13,   5,  -2,   4,  17,  17,
            13,   9,  -3,  -7,  -7,  -8,   3,  -1,
            4,   7,  -6,   1,   0,  -5,  -1,  -8,
            13,   8,   8,  10,  13,   0,   2,  -7,
            0,   0,   0,   0,   0,   0,   0,   0
        ];

        #[rustfmt::skip]
        let mg_knight_table = [
            -167, -89, -34, -49,  61, -97, -15, -107,
            -73, -41,  72,  36,  23,  62,   7,  -17,
            -47,  60,  37,  65,  84, 129,  73,   44,
            -9,  17,  19,  53,  37,  69,  18,   22,
            -13,   4,  16,  13,  28,  19,  21,   -8,
            -23,  -9,  12,  10,  19,  17,  25,  -16,
            -29, -53, -12,  -3,  -1,  18, -14,  -19,
            -105, -21, -58, -33, -17, -28, -19,  -23,
        ];

        #[rustfmt::skip]
        let eg_knight_table = [
            -58, -38, -13, -28, -31, -27, -63, -99,
            -25,  -8, -25,  -2,  -9, -25, -24, -52,
            -24, -20,  10,   9,  -1,  -9, -19, -41,
            -17,   3,  22,  22,  22,  11,   8, -18,
            -18,  -6,  16,  25,  16,  17,   4, -18,
            -23,  -3,  -1,  15,  10,  -3, -20, -22,
            -42, -20, -10,  -5,  -2, -20, -23, -44,
            -29, -51, -23, -15, -22, -18, -50, -64,
        ];

        #[rustfmt::skip]
        let mg_bishop_table = [
            -29,   4, -82, -37, -25, -42,   7,  -8,
            -26,  16, -18, -13,  30,  59,  18, -47,
            -16,  37,  43,  40,  35,  50,  37,  -2,
            -4,   5,  19,  50,  37,  37,   7,  -2,
            -6,  13,  13,  26,  34,  12,  10,   4,
            0,  15,  15,  15,  14,  27,  18,  10,
            4,  15,  16,   0,   7,  21,  33,   1,
            -33,  -3, -14, -21, -13, -12, -39, -21,
        ];

        #[rustfmt::skip]
        let eg_bishop_table = [
            -14, -21, -11,  -8, -7,  -9, -17, -24,
            -8,  -4,   7, -12, -3, -13,  -4, -14,
            2,  -8,   0,  -1, -2,   6,   0,   4,
            -3,   9,  12,   9, 14,  10,   3,   2,
            -6,   3,  13,  19,  7,  10,  -3,  -9,
            -12,  -3,   8,  10, 13,   3,  -7, -15,
            -14, -18,  -7,  -1,  4,  -9, -15, -27,
            -23,  -9, -23,  -5, -9, -16,  -5, -17,
        ];

        #[rustfmt::skip]
        let mg_rook_table = [
            32,  42,  32,  51, 63,  9,  31,  43,
            27,  32,  58,  62, 80, 67,  26,  44,
            -5,  19,  26,  36, 17, 45,  61,  16,
            -24, -11,   7,  26, 24, 35,  -8, -20,
            -36, -26, -12,  -1,  9, -7,   6, -23,
            -45, -25, -16, -17,  3,  0,  -5, -33,
            -44, -16, -20,  -9, -1, 11,  -6, -71,
            -19, -13,   1,  17, 16,  7, -37, -26,
        ];

        #[rustfmt::skip]
        let eg_rook_table = [
            13, 10, 18, 15, 12,  12,   8,   5,
            11, 13, 13, 11, -3,   3,   8,   3,
            7,  7,  7,  5,  4,  -3,  -5,  -3,
            4,  3, 13,  1,  2,   1,  -1,   2,
            3,  5,  8,  4, -5,  -6,  -8, -11,
            -4,  0, -5, -1, -7, -12,  -8, -16,
            -6, -6,  0,  2, -9,  -9, -11,  -3,
            -9,  2,  3, -1, -5, -13,   4, -20,
        ];

        #[rustfmt::skip]
        let mg_queen_table = [
            -28,   0,  29,  12,  59,  44,  43,  45,
            -24, -39,  -5,   1, -16,  57,  28,  54,
            -13, -17,   7,   8,  29,  56,  47,  57,
            -27, -27, -16, -16,  -1,  17,  -2,   1,
            -9, -26,  -9, -10,  -2,  -4,   3,  -3,
            -14,   2, -11,  -2,  -5,   2,  14,   5,
            -35,  -8,  11,   2,   8,  15,  -3,   1,
            -1, -18,  -9,  10, -15, -25, -31, -50,
        ];

        #[rustfmt::skip]
        let eg_queen_table = [
            -9,  22,  22,  27,  27,  19,  10,  20,
            -17,  20,  32,  41,  58,  25,  30,   0,
            -20,   6,   9,  49,  47,  35,  19,   9,
            3,  22,  24,  45,  57,  40,  57,  36,
            -18,  28,  19,  47,  31,  34,  39,  23,
            -16, -27,  15,   6,   9,  17,  10,   5,
            -22, -23, -30, -16, -16, -23, -36, -32,
            -33, -28, -22, -43,  -5, -32, -20, -41,
        ];

        #[rustfmt::skip]
        let mg_king_table = [
            -65,  23,  16, -15, -56, -34,   2,  13,
            29,  -1, -20,  -7,  -8,  -4, -38, -29,
            -9,  24,   2, -16, -20,   6,  22, -22,
            -17, -20, -12, -27, -30, -25, -14, -36,
            -49,  -1, -27, -39, -46, -44, -33, -51,
            -14, -14, -22, -46, -44, -30, -15, -27,
            1,   7,  -8, -64, -43, -16,   9,   8,
            -15,  36,  12, -54,   8, -28,  24,  14,
        ];

        #[rustfmt::skip]
        let eg_king_table = [
            -74, -35, -18, -18, -11,  15,   4, -17,
            -12,  17,  14,  17,  17,  38,  23,  11,
            10,  17,  23,  15,  20,  45,  44,  13,
            -8,  22,  24,  27,  26,  33,  26,   3,
            -18,  -4,  21,  24,  27,  23,   9, -11,
            -19,  -3,  11,  21,  23,  16,   7,  -9,
            -27, -11,   4,  13,  14,   4,  -5, -17,
            -53, -34, -21, -11, -28, -14, -24, -43
        ];

        let mut psts = [Score::default(); NUM_PST_PARAMS];

        // Helper to copy a table into the flat array
        let mut copy_pst = |piece: Piece, mg: [i32; 64], eg: [i32; 64]| {
            for i in 0..64 {
                let idx = (piece.index() * 64) + i;
                psts[idx] = Score::new(mg[i], eg[i]);
            }
        };

        copy_pst(Piece::Pawn, mg_pawn_table, eg_pawn_table);
        copy_pst(Piece::Knight, mg_knight_table, eg_knight_table);
        copy_pst(Piece::Bishop, mg_bishop_table, eg_bishop_table);
        copy_pst(Piece::Rook, mg_rook_table, eg_rook_table);
        copy_pst(Piece::Queen, mg_queen_table, eg_queen_table);
        copy_pst(Piece::King, mg_king_table, eg_king_table);

        Self {
            material: [
                Score::new(82, 94),    // Pawn
                Score::new(337, 281),  // Knight
                Score::new(365, 297),  // Bishop
                Score::new(477, 512),  // Rook
                Score::new(1025, 936), // Queen
            ],
            bishop_pair_bonus: Score::new(20, 50),
            castling_bonus: Score::new(30, 0),
            pawn_shield_full: Score::new(25, 0),
            pawn_shield_partial: Score::new(10, 0),
            open_file_penalty: Score::new(-20, 0),
            semi_open_file_penalty: Score::new(-10, 0),
            isolated_penalty: Score::new(-10, -20),
            doubled_penalty: Score::new(-10, -20),
            backward_penalty: Score::new(-5, -10),
            connected_bonus: Score::new(5, 10),
            passed_pawn_scores: [
                Score::new(0, 0),
                Score::new(5, 10),
                Score::new(10, 20),
                Score::new(20, 40),
                Score::new(40, 80),
                Score::new(80, 160),
                Score::new(150, 300),
                Score::new(0, 0),
            ],
            rook_open_file_bonus: Score::new(30, 15),
            rook_semi_file_bonus: Score::new(15, 10),
            knight_outpost_bonus: Score::new(30, 20),
            mobility: [
                Score::new(1, 2),
                Score::new(4, 4),
                Score::new(4, 4),
                Score::new(3, 6),
                Score::new(2, 4),
            ],
            psts,
        }
    }
}

impl TunableParams {
    /// Helper to get a weight by its LOGICAL index (from constants above).
    pub fn get_weight(&self, index: usize) -> Score {
        if index >= PST_START {
            let offset = index - PST_START;
            return self.psts[offset];
        }

        match index {
            MATERIAL_PAWN => self.material[0],
            MATERIAL_KNIGHT => self.material[1],
            MATERIAL_BISHOP => self.material[2],
            MATERIAL_ROOK => self.material[3],
            MATERIAL_QUEEN => self.material[4],
            BISHOP_PAIR_BONUS => self.bishop_pair_bonus,
            CASTLING_BONUS => self.castling_bonus,
            PAWN_SHIELD_FULL => self.pawn_shield_full,
            PAWN_SHIELD_PARTIAL => self.pawn_shield_partial,
            OPEN_FILE_PENALTY => self.open_file_penalty,
            SEMI_OPEN_FILE_PENALTY => self.semi_open_file_penalty,
            ISOLATED_PENALTY => self.isolated_penalty,
            DOUBLED_PENALTY => self.doubled_penalty,
            BACKWARD_PENALTY => self.backward_penalty,
            CONNECTED_BONUS => self.connected_bonus,
            ROOK_OPEN_FILE_BONUS => self.rook_open_file_bonus,
            ROOK_SEMI_FILE_BONUS => self.rook_semi_file_bonus,
            KNIGHT_OUTPOST_BONUS => self.knight_outpost_bonus,
            i if (PASSED_PAWN_START..PASSED_PAWN_START + 8).contains(&i) => {
                self.passed_pawn_scores[i - PASSED_PAWN_START]
            }
            _ => Score::default(),
        }
    }

    pub fn get_mobility_weight(&self, piece_idx: usize) -> Score {
        self.mobility[piece_idx]
    }

    // SPSA / OPTIMIZER CONVERSION

    pub fn to_vector(&self) -> Vec<f64> {
        let mut vec = Vec::with_capacity(SPSA_VECTOR_SIZE);

        // Helper to push a Score
        let mut push_score = |s: Score| {
            vec.push(s.mg as f64);
            vec.push(s.eg as f64);
        };

        for s in self.material {
            push_score(s);
        }
        // Standard Features
        push_score(self.bishop_pair_bonus);
        push_score(self.castling_bonus);
        push_score(self.pawn_shield_full);
        push_score(self.pawn_shield_partial);
        push_score(self.open_file_penalty);
        push_score(self.semi_open_file_penalty);
        push_score(self.isolated_penalty);
        push_score(self.doubled_penalty);
        push_score(self.backward_penalty);
        push_score(self.connected_bonus);
        for s in self.passed_pawn_scores {
            push_score(s);
        }
        push_score(self.rook_open_file_bonus);
        push_score(self.rook_semi_file_bonus);
        push_score(self.knight_outpost_bonus);

        //  PSTs
        for score in self.psts {
            push_score(score);
        }

        //  Mobility
        for s in self.mobility {
            push_score(s);
        }

        vec
    }

    pub fn from_vector(vec: &[f64]) -> Self {
        let mut params = Self::default();
        let mut idx = 0;

        // Helper to read a Score
        let mut read_score = || -> Score {
            let mg = vec[idx] as i32;
            let eg = vec[idx + 1] as i32;
            idx += 2;
            Score::new(mg, eg)
        };

        // Material values
        for i in 0..5 {
            params.material[i] = read_score();
        }

        //  Standard Features
        params.bishop_pair_bonus = read_score();
        params.castling_bonus = read_score();
        params.pawn_shield_full = read_score();
        params.pawn_shield_partial = read_score();
        params.open_file_penalty = read_score();
        params.semi_open_file_penalty = read_score();
        params.isolated_penalty = read_score();
        params.doubled_penalty = read_score();
        params.backward_penalty = read_score();
        params.connected_bonus = read_score();
        for i in 0..8 {
            params.passed_pawn_scores[i] = read_score();
        }
        params.rook_open_file_bonus = read_score();
        params.rook_semi_file_bonus = read_score();
        params.knight_outpost_bonus = read_score();

        //  PSTs
        for i in 0..NUM_PST_PARAMS {
            params.psts[i] = read_score();
        }

        //  Mobility
        for i in 0..5 {
            params.mobility[i] = read_score();
        }

        params
    }

    /// Save to TOML File
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> miette::Result<()> {
        let toml_string = toml::to_string_pretty(self).into_diagnostic()?;
        fs::write(path, toml_string).into_diagnostic()?;
        Ok(())
    }

    /// Load from a TOML File
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> miette::Result<Self> {
        let file_content = fs::read_to_string(path).into_diagnostic()?;
        let params: Self = toml::from_str(&file_content).into_diagnostic()?;
        Ok(params)
    }
}
