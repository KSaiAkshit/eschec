use std::sync::LazyLock;

use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::prelude::*;

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
            Piece::all_pieces().for_each(|piece| {
                for square in 0..NUM_SQUARES {
                    keys.pieces[side.index()][piece.index()][square] = rng.random();
                }
            })
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

    Piece::all().for_each(|(piece, side)| {
        let mut piece_bb = *board.positions.get_piece_bb(side, piece);
        while let Some(sq) = piece_bb.pop_lsb() {
            hash ^= ZOBRIST.pieces[side.index()][piece.index()][sq as usize];
        }
    });

    hash ^= ZOBRIST.castling[board.castling_rights.0 as usize];

    if let Some(ep_sq) = board.enpassant_square {
        let opponent_pawns = board.positions.get_piece_bb(board.stm.flip(), Piece::Pawn);

        let legal_ep_capture_sq = MOVE_TABLES.get_pawn_attacks(ep_sq.index(), board.stm);

        if (*opponent_pawns & legal_ep_capture_sq).any() {
            hash ^= ZOBRIST.en_passant_file[ep_sq.col()];
        }
    }

    if board.stm == Side::Black {
        hash ^= ZOBRIST.black_to_move;
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::KIWIPETE;
    #[test]
    fn different_hash() {
        let legal_ep_fen = "4k3/8/8/8/3pP3/8/8/4K3 b - e3 0 1";
        let board = Board::from_fen(legal_ep_fen);

        let hash1 = calculate_hash(&board);

        let illegal_ep_fen = "4k3/8/8/8/3pP3/8/8/4K3 w - - 0 1";
        let board = Board::from_fen(illegal_ep_fen);
        let hash2 = calculate_hash(&board);

        assert_ne!(hash1, hash2, "Both should not be the same");
    }

    #[test]
    fn test_zobrist_hash_symmetry() {
        let mut board = Board::new();
        let original_hash = board.hash;

        let mut legal_moves = MoveBuffer::new();
        board.generate_legal_moves(&mut legal_moves, false);

        for mov in legal_moves {
            let move_data = board.make_move(mov).unwrap();

            // The hash MUST change after a move is made.
            assert_ne!(
                board.hash,
                original_hash,
                "Zobrist hash should change after move {}",
                mov.uci()
            );

            board.unmake_move(&move_data).unwrap();

            // The hash MUST be perfectly restored after unmaking the move.
            assert_eq!(
                board.hash,
                original_hash,
                "Zobrist hash was not restored after unmaking move {}",
                mov.uci()
            );
        }
    }

    // helper function
    fn test_hash_symmetry_for_fen(fen: &str) {
        let mut board = Board::from_fen(fen);
        let original_hash = board.hash;

        // Verify initial calculation
        assert_eq!(
            original_hash,
            calculate_hash(&board),
            "Initial hash calculation mismatch for FEN: {fen}"
        );

        let mut legal_moves = MoveBuffer::new();
        board.generate_legal_moves(&mut legal_moves, false);

        if legal_moves.is_empty() {
            return;
        }

        for mov in legal_moves {
            let move_data = board.make_move(mov).unwrap();

            // Verify incremental update in make_move
            assert_eq!(
                board.hash,
                calculate_hash(&board),
                "Incremental hash update mismatch after move {} on FEN '{}'",
                mov.uci(),
                fen
            );

            board.unmake_move(&move_data).unwrap();

            // Verify perfect restoration after unmake_move
            assert_eq!(
                board.hash,
                original_hash,
                "Zobrist hash was not restored after unmaking move {} on FEN '{}'",
                mov.uci(),
                fen
            );
        }
    }

    #[test]
    fn test_startpos_hash_symmetry() {
        test_hash_symmetry_for_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    }

    #[test]
    fn test_kiwipete_hash_symmetry() {
        test_hash_symmetry_for_fen(KIWIPETE);
    }

    #[test]
    fn test_en_passant_hash_symmetry() {
        // Position where white can play e4, creating a possible en passant square on e3 for black.
        test_hash_symmetry_for_fen("rnbqkbnr/pppp1ppp/8/4p3/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        // Position where white can capture en passant.
        test_hash_symmetry_for_fen("rnbqkbnr/pp1p1ppp/8/2pPp3/8/8/PPP1PPPP/RNBQKBNR w KQkq e6 0 3");
    }

    #[test]
    fn test_castling_hash_symmetry() {
        // Position where both sides can castle.
        test_hash_symmetry_for_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
        // Position where only some rights remain
        test_hash_symmetry_for_fen("r3k2r/8/8/8/8/8/8/R3K2R b Kq - 1 1");
    }

    #[test]
    fn test_promotion_hash_symmetry() {
        // Position where white can promote a pawn (with and without capture).
        test_hash_symmetry_for_fen(
            "r3k2r/pPpp1ppp/1b3nbN/nP6/BBP1P3/q4N2/P2P2PP/R2Q1RK1 b kq - 0 1",
        );
    }

    #[test]
    fn test_hash_differs_on_side_to_move() {
        let board_w = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let board_b = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
        assert_ne!(
            board_w.hash, board_b.hash,
            "Hash must differ based on side to move"
        );
    }

    #[test]
    fn test_hash_differs_on_castling_rights() {
        let board_all = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
        let board_no_wq = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w Kk - 0 1");
        assert_ne!(
            board_all.hash, board_no_wq.hash,
            "Hash must differ based on castling rights"
        );
    }

    #[test]
    fn test_hash_differs_on_en_passant_square() {
        // A legal en passant square
        let board_ep =
            Board::from_fen("rnbqkbnr/pp1p1ppp/8/2pPp3/8/8/PPP1PPPP/RNBQKBNR w KQkq e6 0 3");
        // Same position, but no en passant square
        let board_no_ep =
            Board::from_fen("rnbqkbnr/pp1p1ppp/8/2pPp3/8/8/PPP1PPPP/RNBQKBNR w KQkq - 0 3");
        assert_ne!(
            board_ep.hash, board_no_ep.hash,
            "Hash must differ based on en passant square"
        );
    }

    #[test]
    fn test_hash_ignores_illegal_en_passant_square() {
        // This FEN is illegal because the EP square is e3, but there is no white pawn on e4.
        let board_illegal_ep =
            Board::from_fen("rnbqkbnr/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq e3 0 1");
        // This is the corrected version.
        let board_no_ep =
            Board::from_fen("rnbqkbnr/pppp1ppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");

        assert_eq!(
            board_illegal_ep.hash, board_no_ep.hash,
            "Hash should be identical when FEN's EP square is physically impossible"
        );
    }
}
