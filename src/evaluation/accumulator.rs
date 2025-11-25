use crate::prelude::*;
use crate::tuning::params::{PST_START, TunableParams};
use crate::tuning::trace::EvalTrace;

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
        let weight = self.params.get_mobility_weight(piece.index());
        if side == Side::White {
            self.score += weight * count;
        } else {
            self.score -= weight * count;
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
}

impl<'a> EvalAccumulator for TraceAccumulator<'a> {
    #[inline(always)]
    fn add_feature(&mut self, param_idx: usize, side: Side, count: i32) {
        if side == Side::White {
            self.trace.features[param_idx] += count as i8;
        } else {
            self.trace.features[param_idx] -= count as i8;
        }
    }

    #[inline(always)]
    fn add_mobility(&mut self, piece: Piece, side: Side, count: i32) {
        let idx = piece.index();
        // Safety check. Do not touch King
        if idx < 5 {
            if side == Side::White {
                self.trace.mobility[idx] += count as i16;
            } else {
                self.trace.mobility[idx] -= count as i16;
            }
        }
    }

    #[inline(always)]
    fn add_pst(&mut self, piece: Piece, side: Side, sq: usize) {
        let actual_sq = if side == Side::White { sq } else { sq ^ 56 };
        let idx = PST_START + (piece.index() * 64) + actual_sq;

        if side == Side::White {
            self.trace.features[idx] += 1;
        } else {
            self.trace.features[idx] -= 1;
        }
    }

    #[inline(always)]
    fn add_fixed_score(&mut self, score: Score, side: Side) {
        // Fixed scores are not tunable, so they are ignored
        // Dataset generator will calc the total fixed score seperately
    }
}
