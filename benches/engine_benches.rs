use std::{hint::black_box, str::FromStr};

use criterion::{Criterion, criterion_group, criterion_main};
use eschec::{
    Board, KIWIPETE, Square,
    evaluation::{CompositeEvaluator, Evaluator},
    search::Search,
};

fn bench_get_piece_at(c: &mut Criterion) {
    let board = Board::from_fen(KIWIPETE);
    let positions = &board.positions;

    c.bench_function("get_piece_at", |b| {
        b.iter(|| {
            black_box(positions.get_piece_at(&Square::from_str("e1").unwrap()));
            black_box(positions.get_piece_at(&Square::from_str("d5").unwrap()));
            black_box(positions.get_piece_at(&Square::from_str("h7").unwrap()));
            black_box(positions.get_piece_at(&Square::from_str("f3").unwrap()));
        });
    });
}

/// Benchmark for generating all pseudo-legal moves for a position.
/// This measures the raw speed of move generator.
fn bench_move_generation(c: &mut Criterion) {
    let board = Board::from_fen(KIWIPETE);

    c.bench_function("generate_all_moves", |b| {
        b.iter(|| {
            black_box(board.generate_legal_moves());
        })
    });
}

/// Benchmark for the full evaluation function.
/// This will show the impact of optimizing the mobility score.
fn bench_evaluation(c: &mut Criterion) {
    let board = Board::from_fen(KIWIPETE);
    let evaluator = CompositeEvaluator::balanced();

    c.bench_function("evaluate_position", |b| {
        b.iter(|| {
            black_box(evaluator.evaluate(black_box(&board)));
        })
    });
}

/// The is a shallow search on a complex position.
/// This measures how all the components (move gen, make/unmake, evaluation) work together.
fn bench_search(c: &mut Criterion) {
    let board = Board::from_fen(KIWIPETE);
    let evaluator = CompositeEvaluator::balanced();
    // Use a fixed, shallow depth for a stable benchmark.
    let depth = 3;
    let mut search = Search::new(depth);

    c.bench_function(&format!("search_depth_{depth}"), |b| {
        b.iter(|| {
            black_box(search.find_best_move(black_box(&board), black_box(&evaluator)));
        })
    });
}

criterion_group!(
    benches,
    bench_get_piece_at,
    bench_move_generation,
    bench_evaluation,
    bench_search
);

criterion_main!(benches);
