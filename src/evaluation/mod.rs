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
