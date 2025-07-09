use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

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

fn bench_search(c: &mut Criterion) {
    let evaluator = CompositeEvaluator::balanced();
    let depth = 3;
    let mut search = Search::new(depth);

    c.bench_function(&format!("search_depth_{depth}"), |b| {
        b.iter_batched(
            || Board::from_fen(KIWIPETE),
            |board| black_box(search.find_best_move(&board, &evaluator)),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_move_generation,
    bench_evaluation,
    bench_search
);
criterion_main!(benches);
