use crate::{
    evaluation::accumulator::EvalAccumulator,
    prelude::*,
    tuning::params::{
        BISHOP_PAIR_BONUS, MATERIAL_BISHOP, MATERIAL_KNIGHT, MATERIAL_PAWN, MATERIAL_QUEEN,
        MATERIAL_ROOK,
    },
};

pub(crate) fn eval_material(board: &Board, acc: &mut impl EvalAccumulator) {
    let pieces = [
        (Piece::Pawn, MATERIAL_PAWN),
        (Piece::Knight, MATERIAL_KNIGHT),
        (Piece::Bishop, MATERIAL_BISHOP),
        (Piece::Rook, MATERIAL_ROOK),
        (Piece::Queen, MATERIAL_QUEEN),
    ];

    for (piece, param_idx) in pieces {
        let white_count = board.positions.get_piece_bb(Side::White, piece).pop_count();
        let black_count = board.positions.get_piece_bb(Side::Black, piece).pop_count();

        if white_count > 0 {
            acc.add_feature(param_idx, Side::White, white_count as i32);
        }
        if black_count > 0 {
            acc.add_feature(param_idx, Side::Black, black_count as i32);
        }
    }

    // Bishop Pair
    if board
        .positions
        .get_piece_bb(Side::White, Piece::Bishop)
        .pop_count()
        >= 2
    {
        acc.add_feature(BISHOP_PAIR_BONUS, Side::White, 1);
    }

    if board
        .positions
        .get_piece_bb(Side::Black, Piece::Bishop)
        .pop_count()
        >= 2
    {
        acc.add_feature(BISHOP_PAIR_BONUS, Side::Black, 1);
    }
}
