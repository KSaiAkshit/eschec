use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::{cmp::max, hint::black_box};

use eschec::{
    KIWIPETE,
    board::Board,
    evaluation::{CompositeEvaluator, Evaluator},
    search::Search,
};

fn bench_move_generation(c: &mut Criterion) {
    c.bench_function("generate_all_moves", |b| {
        b.iter_batched(
            || Board::from_fen(KIWIPETE),
            |board| black_box(board.generate_legal_moves()),
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
        return evaluator.evaluate(board);
    }

    let pseudo_legal_moves = board.generate_pseudo_legal_moves();
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
        return evaluator.evaluate(board);
    }

    let pseudo_legal_moves = board.generate_pseudo_legal_moves();
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

fn bench_search(c: &mut Criterion) {
    let evaluator = CompositeEvaluator::balanced();
    let depth = 5;
    let mut search = Search::new(depth);

    c.bench_function(&format!("search_depth_{depth}"), |b| {
        b.iter_batched(
            || Board::from_fen(KIWIPETE),
            |board| black_box(search.find_best_move(&board, &evaluator, None)),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_move_generation,
    bench_evaluation,
    bench_alpha_beta_versions,
    bench_search
);
criterion_main!(benches);
