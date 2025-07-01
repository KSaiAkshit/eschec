use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use eschec::board::{Board, components::Square};

fn setup_board() -> Board {
    Board::new()
}

// This is the function that contains the benchmark logic
fn make_unmake_move_cycle(c: &mut Criterion) {
    let mut board = setup_board();
    let from = Square::new(12).unwrap(); // e2
    let to = Square::new(28).unwrap(); // e4

    c.bench_function("make_unmake_move_cycle", |b| {
        // The `b.iter` closure is the code that gets timed.
        b.iter(|| {
            let move_data = board.try_move_with_info(from, to).unwrap();
            board.unmake_move(&move_data).unwrap();

            black_box(&board);
        });
    });
}

criterion_group!(benches, make_unmake_move_cycle);
criterion_main!(benches);
