use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::{hint::black_box, str::FromStr};

use eschec::{
    KIWIPETE,
    board::{
        Board,
        components::{BoardState, Piece, Side, Square},
    },
    moves::move_info::Move,
};

fn setup_board_state() -> BoardState {
    Board::from_fen(KIWIPETE).positions
}

// Board Struct Benchmarks
fn bench_board_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("Board_Ops");
    let from = Square::new(12).unwrap(); // e2
    let to = Square::new(28).unwrap(); // e4
    let mov = Move::new(from.index() as u8, to.index() as u8, Move::QUIET);

    group.bench_function("make_unmake_move_cycle", |b| {
        b.iter_batched(
            || Board::new(), // Setup: create a fresh board
            |mut board| {
                // Timed routine
                let move_data = board.make_move(mov).unwrap();
                board.unmake_move(&move_data).unwrap();
                black_box(&board);
            },
            BatchSize::SmallInput,
        );
    });
}

// BoardState Struct Benchmarks
fn bench_boardstate_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("BoardState_Ops");

    group.bench_function("set_piece", |b| {
        b.iter_batched(
            || (setup_board_state(), Side::White, Piece::Knight, 27),
            |(mut state, side, piece, pos)| black_box(state.set(side, piece, pos).unwrap()),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("capture_piece", |b| {
        b.iter_batched(
            || (setup_board_state(), Side::Black, Piece::Pawn, 53),
            |(mut state, side, piece, pos)| {
                black_box(state.remove_piece(side, piece, pos).unwrap())
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("update_piece_position", |b| {
        b.iter_batched(
            || {
                (
                    setup_board_state(),
                    Piece::Pawn,
                    Side::White,
                    Square::new(28).unwrap(),
                    Square::new(29).unwrap(),
                )
            },
            |(mut state, piece, side, from, to)| {
                black_box(state.move_piece(piece, side, from, to).unwrap());
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("get_piece_at (mailbox)", |b| {
        b.iter_batched(
            || (setup_board_state(), Square::from_str("e4").unwrap()),
            |(state, square)| black_box(state.get_piece_at(&square)),
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_board_ops, bench_boardstate_ops);
criterion_main!(benches);
