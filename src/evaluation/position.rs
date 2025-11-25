use crate::{
    evaluation::accumulator::EvalAccumulator,
    prelude::*,
    tuning::params::{KNIGHT_OUTPOST_BONUS, ROOK_OPEN_FILE_BONUS, ROOK_SEMI_FILE_BONUS},
};

const WHITE_OUTPOST_MASK: BitBoard = BitBoard(0x00007E7E7E000000);
const BLACK_OUTPOST_MASK: BitBoard = BitBoard(0x0000007E7E7E0000);

pub(crate) fn eval_position(board: &Board, acc: &mut impl EvalAccumulator) {
    for side in [Side::White, Side::Black] {
        eval_side_position(board, side, acc);
    }
}

fn eval_side_position(board: &Board, side: Side, acc: &mut impl EvalAccumulator) {
    let opponent = side.flip();
    let friendly_pawns = board.positions.get_piece_bb(side, Piece::Pawn);
    let opponent_pawns = board.positions.get_piece_bb(opponent, Piece::Pawn);
    let side_idx = side.index();
    let opp_idx = opponent.index();

    // Piece Square Tables
    for piece in Piece::all_pieces() {
        let bb = board.positions.get_piece_bb(side, piece);
        for sq in bb.iter_bits() {
            acc.add_pst(piece, side, sq);
        }
    }

    // Knight Outposts
    let knighs = board.positions.get_piece_bb(side, Piece::Knight);
    let outpost_mask = if side == Side::White {
        WHITE_OUTPOST_MASK
    } else {
        BLACK_OUTPOST_MASK
    };

    for knight_sq in (knighs & &outpost_mask).iter_bits() {
        // Check friendly pawn support
        let pawn_support_squares = PAWN_TABLES.pawn_backward_support_masks[side_idx][knight_sq];
        let is_supported = (pawn_support_squares & *friendly_pawns).any();

        if is_supported {
            // Check enemy pawn attacks (squares from which an enemy pawn could attack this knight)
            let pawn_attacks_square = PAWN_TABLES.pawn_backward_support_masks[opp_idx][knight_sq];
            let is_attackable = (pawn_attacks_square & *opponent_pawns).any();

            if !is_attackable {
                acc.add_feature(KNIGHT_OUTPOST_BONUS, side, 1);
            }
        }
    }

    // Rook Open/Semi-Open files
    let rooks = board.positions.get_piece_bb(side, Piece::Rook);
    for rook_sq in rooks.iter_bits() {
        let file_mask = BitBoard(FILE_MASKS[rook_sq % 8]);
        let friendly_pawns_on_file = (file_mask & *friendly_pawns).any();
        let opponent_pawns_on_file = (file_mask & *opponent_pawns).any();

        if !friendly_pawns_on_file {
            if !opponent_pawns_on_file {
                acc.add_feature(ROOK_OPEN_FILE_BONUS, side, 1);
            } else {
                acc.add_feature(ROOK_SEMI_FILE_BONUS, side, 1);
            }
        }
    }
}
