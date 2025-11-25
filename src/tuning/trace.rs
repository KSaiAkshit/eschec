use crate::tuning::params::{self, NUM_TRACE_FEATURES};

// Material Values
pub const MATERIAL_PAWN: usize = 0;
pub const MATERIAL_KNIGHT: usize = 1;
pub const MATERIAL_BISHOP: usize = 2;
pub const MATERIAL_ROOK: usize = 3;
pub const MATERIAL_QUEEN: usize = 4;

// Material
pub const BISHOP_PAIR: usize = 5;

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

// Passed Pawns (8 Ranks)
pub const PASSED_PAWN_START: usize = 15;
// Indices 15..22 are passed pawns

// Position Bonuses
pub const ROOK_OPEN_FILE: usize = 23;
pub const ROOK_SEMI_FILE: usize = 24;
pub const KNIGHT_OUTPOST: usize = 25;

// PSTs (384 params)
// We put PSTs before mobility in the 'features' array to keep i8s together
pub const PST_START: usize = 26;

// Mobility (5 params)
// These are stored in a separate i16 array because counts can exceed 127
pub const MOBILITY_PAWN: usize = 0;
pub const MOBILITY_KNIGHT: usize = 1;
pub const MOBILITY_BISHOP: usize = 2;
pub const MOBILITY_ROOK: usize = 3;
pub const MOBILITY_QUEEN: usize = 4;

#[derive(Debug, Clone)]
pub struct EvalTrace {
    /// Stores all standard features and PSTs.
    /// We use i8 to save RAM (12M positions * ~400 bytes = ~4.8 GB).
    ///
    /// Indexing:
    /// 0..19: Standard Features (Bishop Pair, King Safety, etc.)
    /// 20..403: PSTs (Piece-Square Tables)
    pub features: [i8; NUM_TRACE_FEATURES],

    /// Stores mobility counts.
    /// We use i16 because move counts can exceed 127 (e.g. total queen moves).
    /// Index 0=Pawn, 1=Knight, 2=Bishop, 3=Rook, 4=Queen
    pub mobility: [i16; 5],
}

impl Default for EvalTrace {
    fn default() -> Self {
        Self {
            features: [0; NUM_TRACE_FEATURES],
            mobility: [0; 5],
        }
    }
}

impl EvalTrace {
    /// Returns the index in SPSA vector (Logical Index * 2) for a given Trace Feature Index
    /// Panics if the mapping is invalid
    pub fn map_feature_to_spsa_index(trace_idx: usize) -> usize {
        let logical_idx = match trace_idx {
            MATERIAL_PAWN => params::MATERIAL_PAWN,
            MATERIAL_KNIGHT => params::MATERIAL_KNIGHT,
            MATERIAL_BISHOP => params::MATERIAL_BISHOP,
            MATERIAL_ROOK => params::MATERIAL_ROOK,
            MATERIAL_QUEEN => params::MATERIAL_QUEEN,

            BISHOP_PAIR => params::BISHOP_PAIR_BONUS,

            CASTLING_BONUS => params::CASTLING_BONUS,
            PAWN_SHIELD_FULL => params::PAWN_SHIELD_FULL,
            PAWN_SHIELD_PARTIAL => params::PAWN_SHIELD_PARTIAL,
            OPEN_FILE_PENALTY => params::OPEN_FILE_PENALTY,
            SEMI_OPEN_FILE_PENALTY => params::SEMI_OPEN_FILE_PENALTY,

            ISOLATED_PENALTY => params::ISOLATED_PENALTY,
            DOUBLED_PENALTY => params::DOUBLED_PENALTY,
            BACKWARD_PENALTY => params::BACKWARD_PENALTY,
            CONNECTED_BONUS => params::CONNECTED_BONUS,

            // Passed Pawns
            i if (PASSED_PAWN_START..PASSED_PAWN_START + 8).contains(&i) => {
                params::PASSED_PAWN_START + (i - PASSED_PAWN_START)
            }

            ROOK_OPEN_FILE => params::ROOK_OPEN_FILE_BONUS,
            ROOK_SEMI_FILE => params::ROOK_SEMI_FILE_BONUS,
            KNIGHT_OUTPOST => params::KNIGHT_OUTPOST_BONUS,

            // PSTs
            i if i >= PST_START => params::PST_START + (i - PST_START),
            _ => panic!("Invalid Trace Index: {}", trace_idx),
        };

        logical_idx * 2
    }

    pub fn map_mobility_to_spsa_index(mob_idx: usize) -> usize {
        // Mobility is stored at the end of the params struct/vector
        // We need to find the offset.
        // The vector is: [Features... | Mobility...]
        // Features end at p::NUM_TRACE_FEATURES * 2
        let base = params::NUM_TRACE_FEATURES * 2;
        base + (mob_idx * 2)
    }
}
