use tracing_subscriber::fmt::init;

use crate::board::Board;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    str::{self},
};

// verify symmetry of make_move and unmake_move
fn test_make_unmake_symmetry(fen: &str) {
    init();

    let mut board = Board::from_fen(fen);
    let original_board = board;

    let legal_moves = board.generate_legal_moves(false);

    if legal_moves.is_empty() {
        return;
    }

    for mov in legal_moves {
        let move_data = match board.make_move(mov) {
            Ok(data) => data,
            Err(e) => {
                panic!(
                    "make_move failed for FEN '{fen}', with move {}: {e:?}",
                    mov.uci()
                )
            }
        };
        assert_ne!(
            board,
            original_board,
            "Board state should change after making move {} on FEN {fen}",
            mov.uci()
        );

        if let Err(e) = board.unmake_move(&move_data) {
            panic!(
                "unmake_move failed for FEN '{fen}', with move {}: {e:?}",
                mov.uci()
            )
        }

        assert_eq!(
            board,
            original_board,
            "Board state was not restored after unmaking move {} on FEN {fen}",
            mov.uci()
        );
    }
}

/// Spawns a Stockfish process, communicates with it via UCI, and returns a sorted list of legal moves.
fn get_stockfish_legal_moves(fen: &str) -> Vec<String> {
    let stockfish_path =
        std::env::var("STOCKFISH_PATH").expect("STOCKFISH_PATH environment variable not set");

    let mut child = Command::new(&stockfish_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to spawn Stockfish at '{stockfish_path}': {e}"));

    let mut stdin = child.stdin.take().expect("Failed to open Stockfish stdin");
    let stdout = child
        .stdout
        .take()
        .expect("Failed to open Stockfish stdout");
    let mut reader = BufReader::new(stdout);

    writeln!(stdin, "position fen {fen}").expect("Failed to write to Stockfish stdin");
    writeln!(stdin, "go perft 1").expect("Failed to write to Stockfish stdin");

    let mut moves = Vec::new();
    let mut buffer = String::new();

    loop {
        buffer.clear();
        if reader.read_line(&mut buffer).unwrap_or(0) == 0 {
            break;
        }

        let line = buffer.trim();

        if line.starts_with("info") {
            continue;
        }

        if line.starts_with("Nodes searched:") {
            break;
        }

        if let Some((uci_move, _)) = line.split_once(':') {
            moves.push(uci_move.to_string());
        }
    }

    writeln!(stdin, "quit").ok();
    child
        .wait()
        .expect("Stockfish process terminated uncleanly");

    // Sort the moves for consistent comparison
    moves.sort_unstable();
    moves
}

/// A helper function that compares our engine's move generation against Stockfish's.
/// It now compares sorted lists for better debug output.
fn assert_moves_match_stockfish(fen: &str) {
    // 1. Generate and sort moves with our engine
    let board = Board::from_fen(fen);
    let mut our_moves: Vec<String> = board
        .generate_legal_moves(false)
        .into_iter()
        .map(|m| m.uci())
        .collect();
    our_moves.sort_unstable();

    // 2. Get sorted "ground truth" moves directly from Stockfish
    let stockfish_moves = get_stockfish_legal_moves(fen);

    // 3. Compare the sorted lists. `assert_eq!` will provide a clean diff on failure.
    assert_eq!(
        our_moves, stockfish_moves,
        "\nMove generation mismatch for FEN: '{fen}'"
    );
}

#[test]
fn test_make_unmake_startpos() {
    test_make_unmake_symmetry("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
}

#[test]
fn test_make_unmake_kiwipete() {
    test_make_unmake_symmetry(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    );
}

#[test]
fn test_make_unmake_en_passant() {
    // A position with a valid en passant square.
    test_make_unmake_symmetry("rnbqkbnr/pp1p1ppp/8/2pPp3/8/8/PPP1PPPP/RNBQKBNR w KQkq e6 0 3");
}

#[test]
fn test_make_unmake_promotion() {
    // A position where white can promote a pawn (with and without capture).
    test_make_unmake_symmetry("r3k2r/pPpp1ppp/1b3nbN/nP6/BBP1P3/q4N2/P2P2PP/R2Q1RK1 b kq - 0 1");
}

#[test]
fn test_make_unmake_castling() {
    // A position where both sides can castle.
    test_make_unmake_symmetry("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
}

#[test]
fn test_make_unmake_in_check() {
    // A position where the king is in check and must respond.
    test_make_unmake_symmetry("rnb1kbnr/pppp1ppp/8/4p3/4P2q/8/PPPP1PPP/RNBQKBNR w KQkq - 2 3");
}

#[test]
fn test_make_unmake_double_check() {
    // A position where the king is in double check.
    test_make_unmake_symmetry("rnb1kbnr/pppp1ppp/8/8/3r4/3B4/PPP1PPPP/RN1QK1NR w KQkq - 0 5");
}

#[test]
fn test_start_pos() {
    assert_moves_match_stockfish("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
}

#[test]
fn test_kiwipete_position() {
    assert_moves_match_stockfish(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    );
}

#[test]
fn test_pawn_captures_and_pushes() {
    assert_moves_match_stockfish("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1");
}

#[test]
fn test_en_passant_white() {
    // This test is tricky. We need to create the board state that *allows* en passant.
    // FEN alone isn't enough, as the en passant square is a separate field.
    // We can construct the FEN manually to include the en passant target square.
    // After white e4, black d5, white e5, black f5, the FEN would be:
    // rnbqkbnr/ppppp1pp/8/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3
    // White can now play exf6 en passant.
    assert_moves_match_stockfish("rnbqkbnr/ppppp1pp/8/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3");
}

#[test]
fn test_promotion_with_capture() {
    assert_moves_match_stockfish("rnb2bnr/pppkPppp/8/8/8/8/PPPP1PPP/RNBQKBNR w KQ - 1 5");
}

#[test]
fn test_castling_all_rights() {
    assert_moves_match_stockfish("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
}

#[test]
fn test_castling_blocked() {
    assert_moves_match_stockfish("r3k1nr/p2ppppp/8/8/8/8/P2PPPPP/R1B1K2R w KQkq - 0 1");
}

#[test]
fn test_no_castling_through_check() {
    assert_moves_match_stockfish("1r2k2r/8/8/8/8/8/8/R3K2R w KQk - 0 1");
}

#[test]
fn test_no_castling_while_in_check() {
    assert_moves_match_stockfish("r3k2r/pp1ppppp/8/2b5/8/8/PPP1PPPP/R3K2R w KQkq - 0 1");
}

#[test]
fn test_pinned_piece_cannot_move() {
    assert_moves_match_stockfish("rnbqk1nr/1p1p1pbp/4p1p1/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 4");
}

#[test]
fn test_pinned_piece_can_move_along_ray() {
    assert_moves_match_stockfish("4k3/4r3/8/8/8/8/4R3/4K3 w - - 0 1");
}

#[test]
fn test_must_move_out_of_check() {
    assert_moves_match_stockfish("rnbqkbnr/pppp1ppp/8/8/4r3/8/PPPPPPPP/RNBQKBNR w KQkq - 0 2");
}

#[test]
fn test_double_check_only_king_moves() {
    assert_moves_match_stockfish("rnb1kbnr/pppp1ppp/8/8/3r4/3B4/PPP1PPPP/RN1QK1NR w KQkq - 0 5");
}

#[test]
fn test_stalemate_position() {
    assert_moves_match_stockfish("8/8/8/8/8/8/5Q2/7k b - - 0 1");
}

#[test]
fn test_checkmate_position() {
    assert_moves_match_stockfish("5rk1/p4ppp/8/1p1p4/3P3q/1P2r3/P5PP/2R2Q1K b - - 1 27");
}
