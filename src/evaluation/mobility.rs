use crate::{evaluation::accumulator::EvalAccumulator, prelude::*};

pub(crate) fn eval_mobility(board: &Board, acc: &mut impl EvalAccumulator) {
    let mut buffer = MoveBuffer::new();

    // White Mobility
    board.generate_pseudo_legal_moves(&mut buffer, Some(Side::White));

    for m in &buffer {
        let from = m.from_sq();

        if let Some(piece) = board.get_piece_at(from) {
            // We generally don't count King or Pawn mobility in this specific term
            if piece != Piece::King && piece != Piece::Pawn {
                acc.add_mobility(piece, Side::White, 1);
            }
        }
    }

    buffer.clear();

    // Black Mobility
    board.generate_pseudo_legal_moves(&mut buffer, Some(Side::Black));

    for m in &buffer {
        let from = m.from_sq();

        if let Some(piece) = board.get_piece_at(from)
            && piece != Piece::King
            && piece != Piece::Pawn
        {
            acc.add_mobility(piece, Side::Black, 1);
        }
    }
}
