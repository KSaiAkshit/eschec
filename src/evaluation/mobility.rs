use crate::{evaluation::accumulator::EvalAccumulator, prelude::*};

pub(crate) fn eval_mobility(board: &Board, acc: &mut impl EvalAccumulator) {
    let mut buffer = MoveBuffer::new();

    // White Mobility
    board.generate_pseudo_legal_moves(&mut buffer, Some(Side::White));

    let mut move_counts = [0u8; 64];

    for m in &buffer {
        let from = m.from_sq().index();
        move_counts[from] += 1;
    }

    for piece in [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen] {
        let bb = board.positions.get_piece_bb(Side::White, piece);
        for sq in bb.iter_bits() {
            let count = move_counts[sq];
            acc.add_mobility(piece, Side::White, count as i32);
        }
    }

    buffer.clear();

    // Black Mobility
    board.generate_pseudo_legal_moves(&mut buffer, Some(Side::Black));
    move_counts.fill(0);

    for m in &buffer {
        let from = m.from_sq().index();
        move_counts[from] += 1;
    }

    for piece in [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen] {
        let bb = board.positions.get_piece_bb(Side::Black, piece);
        for sq in bb.iter_bits() {
            let count = move_counts[sq];
            acc.add_mobility(piece, Side::Black, count as i32);
        }
    }
}
