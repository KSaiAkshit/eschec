//! An example demonstrating and comparing different strategies for
//! updating the `all_sides` bitboard in `BoardState`.
//!
//! This file is preserved for documentation and performance analysis purposes.
//! The main library uses the "Incremental Update" method, which was found
//! to be the most performant.
//!
//! To run this example: `cargo run --example board_update_strategies`
//! To run with SIMD enabled: `cargo run --example board_update_strategies --features simd`

#![allow(dead_code)]
#![cfg_attr(feature = "simd", feature(portable_simd))]

use eschec::board::components::{BitBoard, BoardState, Side};
use std::hint::black_box;
use std::time::Instant;

// This helper function creates a BoardState in the starting position.
fn setup_board_state() -> BoardState {
    let board = eschec::Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    board.positions
}

// Strategy 1: Scalar Recalculation (Naive aproach)
fn update_all_sides_scalar_recalc(board_state: &mut BoardState) {
    let white_pieces = board_state.get_colored_pieces(Side::White);
    board_state.get_side_bb_mut(Side::White).0 = white_pieces[0].0
        | white_pieces[1].0
        | white_pieces[2].0
        | white_pieces[3].0
        | white_pieces[4].0
        | white_pieces[5].0;

    let black_pieces = board_state.get_colored_pieces(Side::Black);
    board_state.get_side_bb_mut(Side::Black).0 = black_pieces[0].0
        | black_pieces[1].0
        | black_pieces[2].0
        | black_pieces[3].0
        | black_pieces[4].0
        | black_pieces[5].0;
}

// Strategy 2: Fold Recalculation (Should be the same as above)
fn update_all_sides_fold_recalc(board_state: &mut BoardState) {
    let white_pieces = board_state.get_colored_pieces(Side::White);
    board_state.get_side_bb_mut(Side::White).0 =
        white_pieces.iter().fold(BitBoard(0), |acc, &bb| acc | bb).0;

    let black_pieces = board_state.get_colored_pieces(Side::Black);
    board_state.get_side_bb_mut(Side::Black).0 =
        black_pieces.iter().fold(BitBoard(0), |acc, &bb| acc | bb).0;
}

// Strategy 3: SIMD Recalculation (CPU Magic!)
#[cfg(feature = "simd")]
fn update_all_sides_simd_recalc(board_state: &mut BoardState) {
    use std::simd::{num::SimdUint, u64x4};

    let white_pieces = board_state.get_colored_pieces(Side::White);
    let a: &[u64; 4] = &[
        white_pieces[0].0,
        white_pieces[1].0,
        white_pieces[2].0,
        white_pieces[3].0,
    ];
    let vec1 = u64x4::from_slice(a);
    let partial_or = vec1.reduce_or();
    board_state.get_side_bb_mut(Side::White).0 = partial_or | white_pieces[4].0 | white_pieces[5].0;

    let black_pieces = board_state.get_colored_pieces(Side::Black);
    let a: &[u64; 4] = &[
        black_pieces[0].0,
        black_pieces[1].0,
        black_pieces[2].0,
        black_pieces[3].0,
    ];
    let vec1 = u64x4::from_slice(a);
    let partial_or = vec1.reduce_or();
    board_state.get_side_bb_mut(Side::Black).0 = partial_or | black_pieces[4].0 | black_pieces[5].0;
}

// Strategy 4: Incremental Update (Better algo)
fn incremental_update(board_state: &mut BoardState, from: usize, to: usize) {
    board_state.get_side_bb_mut(Side::White).capture(from);
    board_state.get_side_bb_mut(Side::White).set(to);
}

fn main() {
    println!("Running a simple performance comparison of board update strategies.");
    println!("This is not a formal benchmark, but a demonstration of relative speed.");
    let iterations = 1_000_000;

    // A small set of common opening moves to simulate different inputs.
    let moves_to_test = [
        (12, 28), // e2-e4
        (51, 35), // d7-d5
        (6, 21),  // g1-f3
        (57, 42), // b8-c6
        (3, 27),  // d1-d4
    ];
    let num_moves = moves_to_test.len() as u32;

    let mut board_state = setup_board_state();
    let start = Instant::now();
    for _ in 0..iterations {
        update_all_sides_scalar_recalc(&mut board_state);
        black_box(&board_state);
    }
    println!("Scalar Recalc:        {:?}", start.elapsed() / iterations);

    let mut board_state = setup_board_state();
    let start = Instant::now();
    for _ in 0..iterations {
        update_all_sides_fold_recalc(&mut board_state);
        black_box(&board_state);
    }
    println!("Fold Recalc:          {:?}", start.elapsed() / iterations);

    #[cfg(feature = "simd")]
    {
        let mut board_state = setup_board_state();
        let start = Instant::now();
        for _ in 0..iterations {
            update_all_sides_simd_recalc(&mut board_state);
            black_box(&board_state);
        }
        println!("SIMD Recalc:          {:?}", start.elapsed() / iterations);
    }
    #[cfg(not(feature = "simd"))]
    {
        println!("SIMD Recalc:          (disabled, compile with --features simd)");
    }

    let mut board_state = setup_board_state();
    let start = Instant::now();
    for i in 0..iterations {
        let (from, to) = moves_to_test[(i % num_moves) as usize];

        incremental_update(&mut board_state, from, to);
        // "Undo" the move to keep the board state consistent
        incremental_update(&mut board_state, to, from);

        black_box(&board_state);
    }
    println!(
        "Incremental Update:   {:?} ns",
        start.elapsed().as_nanos() / (iterations * 2) as u128
    );
}
