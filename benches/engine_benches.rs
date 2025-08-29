use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::{cmp::max, hint::black_box};

use eschec::{
    board::Board,
    consts::KIWIPETE,
    evaluation::{CompositeEvaluator, Evaluator},
    moves::move_gen,
    prelude::MoveBuffer,
    search::{
        Search,
        move_ordering::{score_move, sort_moves},
    },
};

fn filter_captures(board: &Board, buffer: &mut MoveBuffer) {
    board.generate_legal_moves(buffer, false);
    buffer.retain(|mv| mv.is_capture());
}

fn gen_captures(board: &Board, buffer: &mut MoveBuffer) {
    move_gen::generate_forcing_moves(board, buffer);
}

fn bench_move_generation(c: &mut Criterion) {
    c.bench_function("generate_all_moves", |b| {
        b.iter_batched(
            || Board::from_fen(KIWIPETE),
            |board| {
                let mut moves = MoveBuffer::new();
                (board.generate_legal_moves(&mut moves, false));
                black_box(moves)
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("generate_retain_captures", |b| {
        b.iter_batched(
            || (Board::from_fen(KIWIPETE), MoveBuffer::new()),
            |(board, mut buffer)| {
                filter_captures(&board, &mut buffer);
                black_box(buffer)
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("generate_forcing_moves", |b| {
        b.iter_batched(
            || (Board::from_fen(KIWIPETE), MoveBuffer::new()),
            |(board, mut buffer)| {
                gen_captures(&board, &mut buffer);
                black_box(buffer)
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_evaluation(c: &mut Criterion) {
    let evaluator = CompositeEvaluator::balanced();
    c.bench_function("evaluate_position", |b| {
        b.iter_batched(
            || Board::from_fen(KIWIPETE),
            |board| black_box(evaluator.evaluate(&board)),
            BatchSize::SmallInput,
        );
    });
}

fn alpha_beta_two_loops(
    board: &Board,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    evaluator: &dyn Evaluator,
) -> i32 {
    if depth == 0 {
        return board.evaluate_position(evaluator);
    }

    let mut pseudo_legal_moves = MoveBuffer::new();
    board.generate_pseudo_legal_moves(&mut pseudo_legal_moves);
    let is_in_check = board.is_in_check(board.stm);
    let mut legal_move_found = false;

    // First loop: Check for game-ending conditions (mate/stalemate)
    for m in &pseudo_legal_moves {
        let mut board_copy = *board;
        if board_copy.make_move(*m).is_ok() && !board_copy.is_in_check(board.stm) {
            legal_move_found = true;
            break;
        }
    }

    if !legal_move_found {
        return if is_in_check {
            -20_000 + depth as i32
        } else {
            0
        };
    }

    // Second loop: Iterate again to evaluate moves
    for m in pseudo_legal_moves {
        let mut board_copy = *board;
        if board_copy.make_move(m).is_err() || board_copy.is_in_check(board.stm) {
            continue;
        }

        let score = -alpha_beta_two_loops(&board_copy, depth - 1, -beta, -alpha, evaluator);
        alpha = max(alpha, score);
        if alpha >= beta {
            return beta; // Pruning
        }
    }
    alpha
}

/// Version 2: An optimized alpha-beta implementation with a single loop.
fn alpha_beta_one_loop(
    board: &Board,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    evaluator: &dyn Evaluator,
) -> i32 {
    if depth == 0 {
        return board.evaluate_position(evaluator);
    }

    let mut pseudo_legal_moves = MoveBuffer::new();
    board.generate_pseudo_legal_moves(&mut pseudo_legal_moves);

    let mut legal_move_found = false;

    for m in pseudo_legal_moves {
        let mut board_copy = *board;
        if board_copy.make_move(m).is_err() || board_copy.is_in_check(board.stm) {
            continue; // Skip illegal moves
        }

        // Check for mate/stalemate after the loop
        if !legal_move_found {
            return if board.is_in_check(board.stm) {
                -20_000 + depth as i32
            } else {
                0
            };
        }
        legal_move_found = true;
        let score = -alpha_beta_one_loop(&board_copy, depth - 1, -beta, -alpha, evaluator);
        alpha = max(alpha, score);
        if alpha >= beta {
            return beta; // Pruning
        }
    }

    alpha
}

fn bench_alpha_beta_versions(c: &mut Criterion) {
    const DEPTH: u8 = 4; // A reasonable depth for a quick benchmark
    let board = Board::from_fen(KIWIPETE);
    let evaluator = CompositeEvaluator::balanced();

    let mut group = c.benchmark_group("AlphaBeta Versions");

    group.bench_function(format!("alpha_beta_two_loops_depth_{DEPTH}"), |b| {
        b.iter(|| {
            alpha_beta_two_loops(
                black_box(&board),
                black_box(DEPTH),
                black_box(i32::MIN + 1),
                black_box(i32::MAX),
                black_box(&evaluator),
            )
        });
    });

    group.bench_function(format!("alpha_beta_one_loop_depth_{DEPTH}"), |b| {
        b.iter(|| {
            alpha_beta_one_loop(
                black_box(&board),
                black_box(DEPTH),
                black_box(i32::MIN + 1),
                black_box(i32::MAX),
                black_box(&evaluator),
            )
        });
    });

    group.finish();
}

fn bench_ordering(c: &mut Criterion) {
    let board = Board::from_fen(KIWIPETE);

    c.bench_function("prng_move_ordering", |b| {
        b.iter_batched(
            || {
                let mut moves = MoveBuffer::new();
                board.generate_legal_moves(&mut moves, false);
                moves
            },
            |mut moves| {
                sort_moves(
                    &board,
                    moves.as_mut_slice(),
                    &[None; 2],
                    None,
                    &[[0; 64]; 64],
                    0xAB_CDEF_ABCD,
                );
                black_box(moves)
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("classical_move_ordering", |b| {
        b.iter_batched(
            || {
                let mut moves = MoveBuffer::new();
                board.generate_legal_moves(&mut moves, false);
                (moves, board)
            },
            |(mut moves, board)| {
                moves.as_mut_slice().sort_unstable_by_key(|&m| {
                    -score_move(&board, m, &[None; 2], None, &[[0; 64]; 64])
                });
                black_box(moves)
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_search_hot_tt(c: &mut Criterion) {
    let evaluator = CompositeEvaluator::balanced();
    let depth = 7;
    let mut search = Search::new(evaluator.clone_box(), depth);

    search.set_emit_info(false);

    let mut group = c.benchmark_group(format!("search_hot_tt_depth_{depth}"));
    let group = group.sample_size(10);

    group.bench_function(format!("search_depth_{depth}"), |b| {
        b.iter_batched(
            || Board::from_fen(KIWIPETE),
            |board| {
                search.clear_tt();
                black_box(search.find_best_move(&board))
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_search_cold_tt(c: &mut Criterion) {
    let evaluator = CompositeEvaluator::balanced();
    let depth = 7;

    let mut group = c.benchmark_group(format!("search_cold_tt_depth_{depth}"));
    let group = group.sample_size(10);

    group.bench_function("asp_off", |b| {
        b.iter_batched(
            || {
                let board = Board::from_fen(KIWIPETE);
                let mut search = Search::new(evaluator.clone_box(), depth);
                search.set_emit_info(false);
                search.set_asp(false);
                (board, search)
            },
            |(board, mut search)| {
                black_box(search.find_best_move(&board));
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("asp_on", |b| {
        b.iter_batched(
            || {
                let board = Board::from_fen(KIWIPETE);
                let mut search = Search::new(evaluator.clone_box(), depth);
                search.set_emit_info(false);
                search.set_asp(true);
                (board, search)
            },
            |(board, mut search)| {
                black_box(search.find_best_move(&board));
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("lmr_off", |b| {
        b.iter_batched(
            || {
                let board = Board::from_fen(KIWIPETE);
                let mut search = Search::new(evaluator.clone_box(), depth);
                search.set_emit_info(false);
                search.set_lmr(false);
                (board, search)
            },
            |(board, mut search)| {
                black_box(search.find_best_move(&board));
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("lmr_on", |b| {
        b.iter_batched(
            || {
                let board = Board::from_fen(KIWIPETE);
                let mut search = Search::new(evaluator.clone_box(), depth);
                search.set_emit_info(false);
                search.set_lmr(true);
                (board, search)
            },
            |(board, mut search)| {
                black_box(search.find_best_move(&board));
            },
            BatchSize::LargeInput,
        );
    });
}

criterion_group!(
    benches,
    bench_move_generation,
    bench_evaluation,
    bench_alpha_beta_versions,
    bench_search_cold_tt,
    bench_search_hot_tt,
    bench_ordering
);
criterion_main!(benches);
