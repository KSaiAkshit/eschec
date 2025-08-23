#![cfg_attr(feature = "simd", feature(portable_simd))]
#![feature(slice_index_methods, likely_unlikely, f16)]

pub mod board;
pub mod comms;
pub mod evaluation;
pub mod moves;
pub mod precomputed;
pub mod prelude;
pub mod search;
pub mod utils;

pub mod consts {
    use crate::prelude::*;

    pub const NUM_SIDES: usize = Side::SIDES.len();
    pub const NUM_PIECES: usize = Piece::PIECES.len();
    pub const NUM_SQUARES: usize = 64;
    pub const NUM_CASTLING_RIGHTS: usize = 16;
    pub const NUM_FILES: usize = 8;
    pub const NUM_RANKS: usize = 8;

    pub const MAX_PLY: usize = 64;
    pub const MAX_MOVES: usize = 256;
    pub const MAX_HASH: usize = 1024;

    pub const MIDGAME_PHASE: i32 = 0;
    pub const ENDGAME_PHASE: i32 = 256;

    pub const TOTAL_PHASE: i32 = 24;

    pub const MATE_SCORE: i32 = 20_000;
    pub const MATE_THRESHOLD: i32 = MATE_SCORE - MAX_PLY as i32;

    pub const FILE_MASKS: [u64; NUM_FILES] = [
        0x0101010101010101, // A
        0x0202020202020202, // B
        0x0404040404040404, // C
        0x0808080808080808, // D
        0x1010101010101010, // E
        0x2020202020202020, // F
        0x4040404040404040, // G
        0x8080808080808080, // H
    ];

    pub const RANK_MASKS: [u64; NUM_RANKS] = [
        0x00000000000000FF, // Rank 1
        0x000000000000FF00, // Rank 2
        0x0000000000FF0000, // Rank 3
        0x00000000FF000000, // Rank 4
        0x000000FF00000000, // Rank 5
        0x0000FF0000000000, // Rank 6
        0x00FF000000000000, // Rank 7
        0xFF00000000000000, // Rank 8
    ];

    pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    pub const KIWIPETE: &str =
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
}
