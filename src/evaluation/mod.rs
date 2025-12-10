use crate::{
    evaluation::accumulator::{ScoreAccumulator, TraceAccumulator},
    prelude::*,
    tuning::{
        params::{TEMPO_BONUS, TunableParams},
        trace::EvalTrace,
    },
};

pub mod score;

pub mod accumulator;
pub mod king_safety;
pub mod material;
pub mod mobility;
pub mod pawn_structure;
pub mod position;

use king_safety::eval_king_safety;
use material::eval_material;
use mobility::eval_mobility;
use pawn_structure::eval_pawn_structure;
use position::eval_position;

// Generic driver function
fn eval_all(board: &Board, acc: &mut impl accumulator::EvalAccumulator) {
    eval_material(board, acc);
    eval_position(board, acc);
    eval_pawn_structure(board, acc);
    eval_mobility(board, acc);
    eval_king_safety(board, acc);

    acc.add_feature(TEMPO_BONUS, board.stm, 1);
}

pub fn evaluate(board: &Board, params: &TunableParams) -> Score {
    let mut acc = ScoreAccumulator {
        params,
        score: Score::default(),
    };

    eval_all(board, &mut acc);

    if board.stm == Side::White {
        acc.score
    } else {
        -acc.score
    }
}

/// Populates the trace and returns the fixed (non-tunable) score
pub fn trace(board: &Board, trace: &mut EvalTrace) -> Score {
    let mut acc = TraceAccumulator {
        trace,
        fixed_score: Score::default(),
    };

    eval_all(board, &mut acc);

    // Returns the fixed score for the tuner to use
    acc.fixed_score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tuning::params::{self, TunableParams};
    use crate::tuning::trace::EvalTrace;

    #[test]
    fn test_eval_consistency() {
        let fen = "r3k2r/pp1q1ppp/2n1bn2/2bp4/3P4/2N1BN2/PP2BPPP/R2Q1RK1 w kq - 0 1";
        let board = Board::try_from_fen(fen).unwrap();

        let mut params = TunableParams::zeros();

        for i in 0..params::KNIGHT_MAX {
            params.mobility_knight[i] = Score::splat(i as i32 * 10);
        }
        params.tempo_bonus = Score::splat(20);

        let score_eval = evaluate(&board, &params);
        println!("Evaluate Score: {}", score_eval);

        let mut trace = EvalTrace::default();
        let fixed_score = super::trace(&board, &mut trace);
        println!("Trace Fixed Score: {}", fixed_score);

        let weights = params.to_vector();
        dbg!(&weights[params::MOBILITY_KNIGHT_START..params::MOBILITY_BISHOP_START]);
        let feature_map: Vec<usize> = (0..params::NUM_TRACE_FEATURES)
            .map(EvalTrace::map_feature_to_spsa_index)
            .collect();

        let mut mg = fixed_score.mg as f64;
        let mut eg = fixed_score.eg as f64;

        for (trace_idx, &count) in trace.features.iter().enumerate() {
            if count != 0 {
                let spsa_idx = feature_map[trace_idx];
                dbg!(spsa_idx);
                mg += dbg!(weights[spsa_idx]) * count as f64;
                eg += dbg!(weights[spsa_idx + 1]) * count as f64;
                dbg!(mg, eg);
            }
        }

        let trace_score_mg = mg as i32;
        let trace_score_eg = eg as i32;

        println!(
            "Reconstructed Trace Score: MG {}, EG {}",
            trace_score_mg, trace_score_eg
        );

        assert_eq!(score_eval.mg, trace_score_mg, "MG Scores do not match!");
        assert_eq!(score_eval.eg, trace_score_eg, "EG Scores do not match!");
    }
}
