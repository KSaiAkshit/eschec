use crate::prelude::*;
use crate::tuning::params::{self, TunableParams};
use crate::tuning::trace::{self, EvalTrace};

/// The core trait that abstracts over "Calculating Score" and "Recording Trace"
/// Evaluators use this trait so they don't need to know if they are playing or tuning
pub trait EvalAccumulator {
    /// Add a standard feature (eg, Bishop Pair, Isolated Pawn)
    /// `param_idx` comes from `[tuning::trace]` consts
    fn add_feature(&mut self, param_idx: usize, side: Side, count: i32);

    /// Add mobility count for a specific piece type
    fn add_mobility(&mut self, piece: Piece, side: Side, count: i32);

    /// Add a Piece-Square Table value
    fn add_pst(&mut self, piece: Piece, side: Side, sq: usize);

    /// Add a fixed score (non-tunable, eg. KingSafety Lookup Table or raw material)
    fn add_fixed_score(&mut self, score: Score, side: Side);
}

/// Score Accumulator - For Playing
pub struct ScoreAccumulator<'a> {
    pub params: &'a TunableParams,
    pub score: Score,
}

impl<'a> EvalAccumulator for ScoreAccumulator<'a> {
    #[inline(always)]
    fn add_feature(&mut self, param_idx: usize, side: Side, count: i32) {
        let weight = self.params.get_weight(param_idx);
        if side == Side::White {
            self.score += weight * count;
        } else {
            self.score -= weight * count;
        }
    }

    #[inline(always)]
    fn add_mobility(&mut self, piece: Piece, side: Side, count: i32) {
        let weight = self.params.get_mobility_weight(piece, count as usize);
        if side == Side::White {
            self.score += weight;
        } else {
            self.score -= weight;
        }
    }

    #[inline(always)]
    fn add_pst(&mut self, piece: Piece, side: Side, sq: usize) {
        // Mirroring is handled here
        // White reads [sq], Black reads [sq ^ 56]
        let actual_sq = if side == Side::White { sq } else { sq ^ 56 };

        let idx = (piece.index() * 64) + actual_sq;
        let weight = self.params.psts[idx];

        if side == Side::White {
            self.score += weight;
        } else {
            self.score -= weight;
        }
    }

    #[inline(always)]
    fn add_fixed_score(&mut self, score: Score, side: Side) {
        if side == Side::White {
            self.score += score;
        } else {
            self.score -= score;
        }
    }
}

/// Trace Accumulator - For Tuning
pub struct TraceAccumulator<'a> {
    pub trace: &'a mut EvalTrace,
    pub fixed_score: Score,
}

impl<'a> EvalAccumulator for TraceAccumulator<'a> {
    #[inline(always)]
    fn add_feature(&mut self, param_idx: usize, side: Side, count: i32) {
        let trace_idx = match param_idx {
            params::MATERIAL_PAWN => trace::MATERIAL_PAWN,
            params::MATERIAL_KNIGHT => trace::MATERIAL_KNIGHT,
            params::MATERIAL_BISHOP => trace::MATERIAL_BISHOP,
            params::MATERIAL_ROOK => trace::MATERIAL_ROOK,
            params::MATERIAL_QUEEN => trace::MATERIAL_QUEEN,

            params::BISHOP_PAIR_BONUS => trace::BISHOP_PAIR,

            params::CASTLING_BONUS => trace::CASTLING_BONUS,
            params::PAWN_SHIELD_FULL => trace::PAWN_SHIELD_FULL,
            params::PAWN_SHIELD_PARTIAL => trace::PAWN_SHIELD_PARTIAL,
            params::OPEN_FILE_PENALTY => trace::OPEN_FILE_PENALTY,
            params::SEMI_OPEN_FILE_PENALTY => trace::SEMI_OPEN_FILE_PENALTY,

            params::ISOLATED_PENALTY => trace::ISOLATED_PENALTY,
            params::DOUBLED_PENALTY => trace::DOUBLED_PENALTY,
            params::BACKWARD_PENALTY => trace::BACKWARD_PENALTY,
            params::CONNECTED_BONUS => trace::CONNECTED_BONUS,

            i if (params::PASSED_PAWN_START..params::PASSED_PAWN_START + 8).contains(&i) => {
                trace::PASSED_PAWN_START + (i - trace::PASSED_PAWN_START)
            }

            params::ROOK_OPEN_FILE_BONUS => trace::ROOK_OPEN_FILE,
            params::ROOK_SEMI_FILE_BONUS => trace::ROOK_SEMI_FILE,
            params::KNIGHT_OUTPOST_BONUS => trace::KNIGHT_OUTPOST,

            params::TEMPO_BONUS => trace::TEMPO_BONUS,

            _ => return,
        };
        if side == Side::White {
            self.trace.features[trace_idx] += count as i8;
        } else {
            self.trace.features[trace_idx] -= count as i8;
        }
    }

    #[inline(always)]
    fn add_mobility(&mut self, piece: Piece, side: Side, count: i32) {
        let count = count as usize;

        let (start_idx, max) = match piece {
            Piece::Knight => (params::MOBILITY_KNIGHT_START, params::KNIGHT_MAX),
            Piece::Bishop => (params::MOBILITY_BISHOP_START, params::BISHOP_MAX),
            Piece::Rook => (params::MOBILITY_ROOK_START, params::ROOK_MAX),
            Piece::Queen => (params::MOBILITY_QUEEN_START, params::QUEEN_MAX),
            _ => return,
        };

        // Clamp count to max-1
        let offset = count.min(max - 1);
        let trace_idx = start_idx + offset;

        if side == Side::White {
            self.trace.features[trace_idx] += 1;
        } else {
            self.trace.features[trace_idx] -= 1;
        }
    }

    #[inline(always)]
    fn add_pst(&mut self, piece: Piece, side: Side, sq: usize) {
        let actual_sq = if side == Side::White { sq } else { sq ^ 56 };

        // In trace.rs, PSTs start at t::PST_START
        // In params.rs, PSTs start at PST_START
        // We calculate the offset relative to the start
        let offset = (piece.index() * 64) + actual_sq;
        let trace_idx = trace::PST_START + offset;

        if side == Side::White {
            self.trace.features[trace_idx] += 1;
        } else {
            self.trace.features[trace_idx] -= 1;
        }
    }

    #[inline(always)]
    fn add_fixed_score(&mut self, score: Score, side: Side) {
        if side == Side::White {
            self.fixed_score += score;
        } else {
            self.fixed_score -= score;
        }
    }
}
