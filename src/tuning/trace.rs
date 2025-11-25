use crate::tuning::params::NUM_TRACE_FEATURES;

// Mobility
pub const BISHOP_PAIR: usize = 0;

// King Safety
pub const CASTLING_BONUS: usize = 1;
pub const PAWN_SHIELD_FULL: usize = 2;
pub const PAWN_SHIELD_PARTIAL: usize = 3;
pub const OPEN_FILE_PENALTY: usize = 4;

// Pawn Structure
pub const ISOLATED_PENALTY: usize = 5;
pub const DOUBLED_PENALTY: usize = 6;
pub const BACKWARD_PENALTY: usize = 7;
pub const CONNECTED_BONUS: usize = 8;

// Passed Pawns (8 Ranks)
pub const PASSED_PAWN_START: usize = 9;
// Indices 9..17 are passed pawns

// Position Bonuses
pub const ROOK_OPEN_FILE: usize = 17;
pub const ROOK_SEMI_FILE: usize = 18;
pub const KNIGHT_OUTPOST: usize = 19;

// PSTs (384 params)
// We put PSTs before mobility in the 'features' array to keep i8s together
pub const PST_START: usize = 20;

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
