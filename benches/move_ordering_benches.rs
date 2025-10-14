use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use eschec::{
    prelude::*,
    search::move_ordering::{MainSearchPolicy, MoveScoringPolicy},
};

const POSITIONS: &[(&str, &str)] = &[
    (
        "Start",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    ),
    (
        "Kiwipete",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    ),
    (
        "Tactical",
        "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1",
    ),
    ("Endgame", "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"),
];

const CUTOFF_POINTS: &[usize] = &[1, 3, 5, 8, 100]; // 100 = "no cutoff"

fn bench_move_scoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("move_scoring");

    for (name, fen) in POSITIONS {
        let board = Board::from_fen(fen);
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);
        let move_count = moves.len();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{name}_({move_count})_moves")),
            &board,
            |b, board| {
                b.iter_batched(
                    || {
                        let mut moves = MoveBuffer::new();
                        board.generate_legal_moves(&mut moves, false);
                        moves
                    },
                    |moves| {
                        // Just score all moves without sorting
                        let scores: Vec<i32> = moves
                            .iter()
                            .map(|&mv| {
                                MainSearchPolicy::score(board, mv, &[None; 2], None, &[[0; 64]; 64])
                            })
                            .collect();
                        black_box(scores)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_current_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_sort");

    for (name, fen) in POSITIONS {
        let board = Board::from_fen(fen);
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);
        let move_count = moves.len();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{name}_({move_count})_moves")),
            &board,
            |b, board| {
                b.iter_batched(
                    || {
                        let mut moves = MoveBuffer::new();
                        board.generate_legal_moves(&mut moves, false);
                        moves
                    },
                    |mut moves| {
                        eschec::search::move_ordering::sort_moves::<MainSearchPolicy>(
                            board,
                            moves.as_mut_slice(),
                            &[None; 2],
                            None,
                            &[[0; 64]; 64],
                            board.hash,
                        );
                        black_box(moves)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_realistic_search_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic_search");

    for (name, fen) in POSITIONS {
        let board = Board::from_fen(fen);
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);
        let move_count = moves.len();

        // Simulate: we search 3 moves on average before beta cutoff
        let avg_cutoff = 3;

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{name}_full_sort")),
            &board,
            |b, board| {
                b.iter_batched(
                    || {
                        let mut moves = MoveBuffer::new();
                        board.generate_legal_moves(&mut moves, false);
                        moves
                    },
                    |mut moves| {
                        // Full sort
                        eschec::search::move_ordering::sort_moves::<MainSearchPolicy>(
                            board,
                            moves.as_mut_slice(),
                            &[None; 2],
                            None,
                            &[[0; 64]; 64],
                            board.hash,
                        );

                        // Simulate searching first N moves
                        let searched: Vec<Move> = moves
                            .as_slice()
                            .iter()
                            .take(avg_cutoff.min(move_count))
                            .copied()
                            .collect();
                        black_box(searched)
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{name}_move_picker_precomputed")),
            &board,
            |b, board| {
                b.iter_batched(
                    || {
                        let mut moves = MoveBuffer::new();
                        board.generate_legal_moves(&mut moves, false);
                        moves
                    },
                    |mut moves| {
                        let mut picker = eschec::search::move_picker::MovePicker::new(
                            board,
                            moves.as_mut_slice(),
                            &[None; 2],
                            None,
                            &[[0; 64]; 64],
                        );

                        // Pick only what we need
                        let mut picked = Vec::with_capacity(avg_cutoff);
                        for _ in 0..avg_cutoff.min(move_count) {
                            if let Some(mv) = picker.next_best() {
                                picked.push(mv);
                            }
                        }
                        black_box(picked)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_move_picker_precomputed(c: &mut Criterion) {
    let mut group = c.benchmark_group("move_picker_precomputed");

    for (name, fen) in POSITIONS {
        let board = Board::from_fen(fen);
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);
        let move_count = moves.len();

        for &cutoff in CUTOFF_POINTS {
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{name}_({move_count}_moves)_cutoff_{cutoff}")),
                &board,
                |b, board| {
                    b.iter_batched(
                        || {
                            let mut moves = MoveBuffer::new();
                            board.generate_legal_moves(&mut moves, false);
                            moves
                        },
                        |mut moves| {
                            let mut picker = eschec::search::move_picker::MovePicker::new(
                                board,
                                moves.as_mut_slice(),
                                &[None; 2],
                                None,
                                &[[0; 64]; 64],
                            );

                            let mut picked = Vec::with_capacity(cutoff);
                            for _ in 0..cutoff.min(move_count) {
                                if let Some(mv) = picker.next_best() {
                                    picked.push(mv);
                                }
                            }
                            black_box(picked)
                        },
                        BatchSize::SmallInput,
                    );
                },
            );
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_current_sort,
    bench_realistic_search_scenario,
    bench_move_picker_precomputed,
    bench_move_scoring,
);
criterion_main!(benches);
