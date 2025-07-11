use std::sync::LazyLock;

use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::{Board, Piece, Side, consts::*};

pub static ZOBRIST: LazyLock<ZobristKeys> = LazyLock::new(ZobristKeys::new);
#[derive(Debug)]
pub struct ZobristKeys {
    /// For each piece type, on each square, for each side
    pub pieces: [[[u64; NUM_SQUARES]; NUM_PIECES]; NUM_SIDES],
    /// For each of the 16 possible castling rights states
    pub castling: [u64; NUM_CASTLING_RIGHTS],
    /// For each of the 8 possible en passant files or none
    pub en_passant_file: [u64; NUM_FILES],
    /// Single key to flip when stm changes
    pub black_to_move: u64,
}

impl Default for ZobristKeys {
    fn default() -> Self {
        Self {
            pieces: [[[0; NUM_SQUARES]; NUM_PIECES]; NUM_SIDES],
            castling: [0; NUM_CASTLING_RIGHTS],
            en_passant_file: [0; NUM_FILES],
            black_to_move: 0,
        }
    }
}

impl ZobristKeys {
    pub fn new() -> Self {
        let mut rng = StdRng::seed_from_u64(1070373321345817214);
        let mut keys = Self {
            black_to_move: rng.random(),
            ..Default::default()
        };

        for side in Side::SIDES {
            for piece in Piece::PIECES {
                for square in 0..NUM_SQUARES {
                    keys.pieces[side.index()][piece.index()][square] = rng.random();
                }
            }
        }

        for i in 0..NUM_CASTLING_RIGHTS {
            keys.castling[i] = rng.random();
        }

        for i in 0..NUM_FILES {
            keys.en_passant_file[i] = rng.random();
        }

        keys
    }
}

pub fn calculate_hash(board: &Board) -> u64 {
    let mut hash = 0;

    Side::SIDES.iter().for_each(|side| {
        Piece::PIECES.iter().for_each(|piece| {
            let mut piece_bb = *board.positions.get_piece_bb(*side, *piece);
            while let Some(sq) = piece_bb.pop_lsb() {
                hash ^= ZOBRIST.pieces[side.index()][piece.index()][sq as usize];
            }
        });
    });

    hash ^= ZOBRIST.castling[board.castling_rights.0 as usize];

    if let Some(ep_sq) = board.enpassant_square {
        hash ^= ZOBRIST.en_passant_file[ep_sq.col()];
    }

    if board.stm == Side::Black {
        hash ^= ZOBRIST.black_to_move;
    }

    hash
}
