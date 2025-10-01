use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::{hint::black_box, str::FromStr};

use eschec::{moves::attack_data::calculate_attack_data, prelude::*};

fn setup_board_state() -> BoardState {
    Board::from_fen(KIWIPETE).positions
}

fn bench_search_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("Search_Loop");
    const DEPTH: u8 = 5; // A small but non-trivial depth

    fn search_copy(board: Board, depth: u8) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut nodes = 0;
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);
        for mv in moves {
            let mut board_copy = board;
            board_copy.make_move(mv).unwrap();
            nodes += search_copy(board_copy, depth - 1);
        }
        nodes
    }
    // Benchmark with the Copy approach
    group.bench_function("search_with_copy", |b| {
        b.iter_batched(
            Board::new,
            |board| {
                black_box(search_copy(board, DEPTH));
            },
            BatchSize::SmallInput,
        );
    });

    fn search_unmake(board: &mut Board, depth: u8) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut nodes = 0;
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);
        for mv in moves {
            let move_info = board.make_move(mv).unwrap();
            nodes += search_unmake(board, depth - 1);
            board.unmake_move(&move_info).unwrap();
        }
        nodes
    }

    // Benchmark with the Make/Unmake approach
    group.bench_function("search_with_unmake", |b| {
        b.iter_batched(
            Board::new,
            |mut board| {
                black_box(search_unmake(&mut board, DEPTH));
            },
            BatchSize::SmallInput,
        );
    });
}

// Board Struct Benchmarks
fn bench_board_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("Board_Ops");
    let from = Square::new(12).unwrap(); // e2
    let to = Square::new(28).unwrap(); // e4
    let mov = Move::new(from.index() as u8, to.index() as u8, Move::QUIET);

    group.bench_function("calculate_attack_data_white", |b| {
        b.iter_batched(
            || Board::from_fen(KIWIPETE),
            |board| {
                black_box(calculate_attack_data(&board, board.stm));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("calculate_attack_data_black", |b| {
        b.iter_batched(
            || Board::from_fen(KIWIPETE),
            |board| {
                black_box(calculate_attack_data(&board, board.stm.flip()));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("make_unmake_move_cycle", |b| {
        b.iter_batched(
            Board::new,
            |mut board| {
                let move_data = board.make_move(mov).unwrap();
                board.unmake_move(&move_data).unwrap();
                black_box(&board);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("static_exhange_eval_see", |b| {
        b.iter_batched(
            || (Board::from_fen(KIWIPETE), Move::new(21, 54, Move::CAPTURE)),
            |(board, mv)| {
                let a = board.static_exchange_evaluation(mv);
                black_box(a);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("board_copy_make_move", |b| {
        b.iter_batched(
            Board::new,
            |board| {
                let mut board_copy = board;
                board_copy.make_move(mov).unwrap();
                black_box(&board_copy);
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
            |(mut state, side, piece, pos)| black_box(state.set(side, piece, pos).is_ok()),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("capture_piece", |b| {
        b.iter_batched(
            || (setup_board_state(), Side::Black, Piece::Pawn, 53),
            |(mut state, side, piece, pos)| black_box(state.remove_piece(side, piece, pos).is_ok()),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("update_piece_position", |b| {
        b.iter_batched(
            || {
                (
                    setup_board_state(),
                    Square::new(28).unwrap(),
                    Square::new(29).unwrap(),
                )
            },
            |(mut state, from, to)| {
                black_box(state.move_piece(from, to).is_ok());
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

criterion_group!(
    benches,
    bench_board_ops,
    bench_boardstate_ops,
    bench_search_loop
);
criterion_main!(benches);
