use crate::{
    evaluation::accumulator::EvalAccumulator,
    prelude::*,
    tuning::params::{
        BACKWARD_PENALTY, CONNECTED_BONUS, DOUBLED_PENALTY, ISOLATED_PENALTY, PASSED_PAWN_START,
    },
};

pub(crate) fn eval_pawn_structure(board: &Board, acc: &mut impl EvalAccumulator) {
    for side in [Side::White, Side::Black] {
        eval_side_pawns(board, side, acc);
    }
}

fn eval_side_pawns(board: &Board, side: Side, acc: &mut impl EvalAccumulator) {
    let side_idx = side.index();
    let friendly_pawns = board.positions.get_piece_bb(side, Piece::Pawn);
    let opponent_pawns = board.positions.get_piece_bb(side.flip(), Piece::Pawn);
    let occupied = board.positions.get_occupied_bb();

    for sq_idx in friendly_pawns.iter_bits() {
        let file = sq_idx % 8;
        let relative_rank = if side == Side::White {
            sq_idx / 8
        } else {
            7 - (sq_idx / 8)
        };

        // Isolated Pawns: No friendly pawns on adjacent files
        if (friendly_pawns & &PAWN_TABLES.pawn_adjacent_files_masks[file]).is_empty() {
            acc.add_feature(ISOLATED_PENALTY, side, 1);
        }

        // Doubled Pawn: More than 1 pawn on this file
        // Note: This counts every pawn on the file as doubled.
        // If there are 2 pawns, this adds the penalty twice thich is standard
        if (friendly_pawns.0 & FILE_MASKS[file]).count_ones() > 1 {
            acc.add_feature(DOUBLED_PENALTY, side, 1);
        }

        // Passed Pawn: No enemy pawns in front or an adjacent files in front
        if (opponent_pawns & &PAWN_TABLES.passed_pawn_blocking_masks[side_idx][sq_idx]).is_empty() {
            // PASSED_PAWN_START is the index for Rank 0. We add relative_rank to get the specific parameter.
            // e.g. Rank 7 passed pawn uses index PASSED_PAWN_START + 7
            acc.add_feature(PASSED_PAWN_START + relative_rank, side, 1);
        }

        // Backward Pawn: Blocked in front AND cannot be supported by friendly pawns
        let is_blocked_in_front =
            (occupied & PAWN_TABLES.pawn_front_square_masks[side_idx][sq_idx]).any();
        let has_backward_support =
            (friendly_pawns & &PAWN_TABLES.pawn_backward_support_masks[side_idx][sq_idx]).any();

        if is_blocked_in_front && !has_backward_support {
            acc.add_feature(BACKWARD_PENALTY, side, 1);
        }

        // Connected Bonus: Friendly Pawns on adjacent files/ranks
        let connected_neighbors = friendly_pawns & &PAWN_TABLES.connected_pawn_masks[sq_idx];
        let count = connected_neighbors.pop_count() as i32;
        if count > 0 {
            acc.add_feature(CONNECTED_BONUS, side, count);
        }
    }
}
