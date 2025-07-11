use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use eschec::{board::components::BitBoard, moves::precomputed::MOVE_TABLES};

// Constants and Setup
const INITIAL_PIECES_BB: BitBoard = BitBoard(0xFFFF00000000FFFF);
const MIDGAME_OCCUPANCY_BB: BitBoard = BitBoard(0x007E8181A5A5FFFF);

// BitBoard Benchmarks
fn bench_bitboard_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("BitBoard_Ops");

    group.bench_function("pop_lsb_loop", |b| {
        b.iter_batched(
            || INITIAL_PIECES_BB,
            |mut bb| {
                while let Some(bit) = bb.pop_lsb() {
                    black_box(bit);
                }
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("pop_msb_loop", |b| {
        b.iter_batched(
            || INITIAL_PIECES_BB,
            |mut bb| {
                while let Some(bit) = bb.pop_msb() {
                    black_box(bit);
                }
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("iterator_fixed", |b| {
        b.iter_batched(
            || INITIAL_PIECES_BB,
            |bb| {
                bb.iter_bits().for_each(|bit| {
                    black_box(bit);
                })
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("iterator_sum", |b| {
        b.iter_batched(
            || INITIAL_PIECES_BB,
            |bb| black_box(bb.iter_bits().sum::<usize>()),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("count_ones", |b| {
        b.iter_batched(
            || INITIAL_PIECES_BB,
            |bb| black_box(bb.count_ones()),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("set_bit", |b| {
        b.iter_batched(
            || (BitBoard(0), 27),
            |(mut bb, pos)| {
                bb.set(black_box(pos));
                black_box(bb);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("capture_bit", |b| {
        b.iter_batched(
            || (INITIAL_PIECES_BB, 12),
            |(mut bb, pos)| {
                bb.capture(black_box(pos));
                black_box(bb);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

// Precomputed Move Table Benchmarks
fn bench_move_lookups(c: &mut Criterion) {
    let mut group = c.benchmark_group("Move_Lookups");

    group.bench_function("knight_moves", |b| {
        b.iter_batched(
            || 27,
            |from_sq| black_box(MOVE_TABLES.knight_moves[from_sq]),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("king_moves", |b| {
        b.iter_batched(
            || 27,
            |from_sq| black_box(MOVE_TABLES.king_moves[from_sq]),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("rook_moves_occupied", |b| {
        b.iter_batched(
            || (27, MIDGAME_OCCUPANCY_BB, MIDGAME_OCCUPANCY_BB),
            |(from_sq, allies, enemies)| {
                black_box(MOVE_TABLES.get_rook_moves(from_sq, allies, enemies));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("bishop_moves_occupied", |b| {
        b.iter_batched(
            || (27, MIDGAME_OCCUPANCY_BB, MIDGAME_OCCUPANCY_BB),
            |(from_sq, allies, enemies)| {
                black_box(MOVE_TABLES.get_bishop_moves(from_sq, allies, enemies));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_bitboard_ops, bench_move_lookups);
criterion_main!(benches);
