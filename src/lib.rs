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

pub mod ansi_colors {
    // Reset
    pub const RESET: &str = "\x1b[0m";

    // Regular colors - foreground
    pub const BLACK: &str = "\x1b[30m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";

    // Bright colors - foreground
    pub const BRIGHT_BLACK: &str = "\x1b[90m";
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
    pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
    pub const BRIGHT_CYAN: &str = "\x1b[96m";
    pub const BRIGHT_WHITE: &str = "\x1b[97m";

    // Regular colors - background
    pub const BG_BLACK: &str = "\x1b[40m";
    pub const BG_RED: &str = "\x1b[41m";
    pub const BG_GREEN: &str = "\x1b[42m";
    pub const BG_YELLOW: &str = "\x1b[43m";
    pub const BG_BLUE: &str = "\x1b[44m";
    pub const BG_MAGENTA: &str = "\x1b[45m";
    pub const BG_CYAN: &str = "\x1b[46m";
    pub const BG_WHITE: &str = "\x1b[47m";

    // Bright colors - background
    pub const BG_BRIGHT_BLACK: &str = "\x1b[100m";
    pub const BG_BRIGHT_RED: &str = "\x1b[101m";
    pub const BG_BRIGHT_GREEN: &str = "\x1b[102m";
    pub const BG_BRIGHT_YELLOW: &str = "\x1b[103m";
    pub const BG_BRIGHT_BLUE: &str = "\x1b[104m";
    pub const BG_BRIGHT_MAGENTA: &str = "\x1b[105m";
    pub const BG_BRIGHT_CYAN: &str = "\x1b[106m";
    pub const BG_BRIGHT_WHITE: &str = "\x1b[107m";
}

pub mod consts {
    use crate::prelude::*;

    pub const NUM_SIDES: usize = Side::SIDES.len();
    pub const NUM_PIECES: usize = Piece::PIECES.len();
    pub const NUM_SQUARES: usize = 64;
    pub const NUM_CASTLING_RIGHTS: usize = 16;
    pub const NUM_FILES: usize = 8;
    pub const NUM_RANKS: usize = 8;

    pub const MAX_PLY: usize = 256;
    pub const MAX_MOVES: usize = 256;
    pub const MAX_HASH: usize = 1024;

    pub const MIDGAME_PHASE: i32 = 0;
    pub const ENDGAME_PHASE: i32 = 256;

    pub const TOTAL_PHASE: i32 = 24;

    pub const STALEMATE_SCORE: i32 = 0;
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
