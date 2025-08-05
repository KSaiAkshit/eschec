use tracing_subscriber::fmt::init;

use crate::prelude::*;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    str::{self},
};

fn bb_from_squares(squares: &[&str]) -> BitBoard {
    let mut bb = BitBoard(0);
    for s in squares {
        let sq = s.parse::<Square>().unwrap();
        bb.set(sq.index());
    }
    bb
}

// Checks a standard pawn double push.
#[test]
fn test_from_uci_simple_quiet_move() {
    let board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let mov = Move::from_uci(&board, "e2e4").unwrap();
    assert_eq!(mov.from_sq(), Square::from_str("e2").unwrap());
    assert_eq!(mov.to_sq(), Square::from_str("e4").unwrap());
    assert_eq!(mov.flags(), Move::DOUBLE_PAWN);
}

// Verifies a simple pawn capture has the correct flag.
#[test]
fn test_from_uci_simple_capture() {
    let board = Board::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1");
    let mov = Move::from_uci(&board, "e4d5").unwrap();
    assert_eq!(mov.flags(), Move::CAPTURE);
}

// Tests a promotion-with-capture to a queen
#[test]
fn test_from_uci_promotion() {
    let board = Board::from_fen("rnbq1bnr/pppkPppp/8/8/8/8/PPPP1PPP/RNBQKBNR w KQ - 1 5");
    let mov = Move::from_uci(&board, "e7d8q").unwrap();
    assert_eq!(mov.flags(), Move::PROMO_QC);
    assert_eq!(mov.promoted_piece(), Some(Piece::Queen));
}

// Checks both kingside and queenside castling
#[test]
fn test_from_uci_castling() {
    let board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
    let mov_ks = Move::from_uci(&board, "e1g1").unwrap();
    assert_eq!(mov_ks.flags(), Move::KING_CASTLE);

    let mov_qs = Move::from_uci(&board, "e1c1").unwrap();
    assert_eq!(mov_qs.flags(), Move::QUEEN_CASTLE);
}

// Ensures the en passant flag is correctly identified
#[test]
fn test_from_uci_en_passant() {
    let board = Board::from_fen("rnbqkbnr/pp1p1ppp/8/2pPp3/8/8/PPP1PPPP/RNBQKBNR w KQkq e6 0 3");
    let mov = Move::from_uci(&board, "d5e6").unwrap();
    assert_eq!(mov.flags(), Move::EN_PASSANT);
}

// test to ensure the final legality check works by trying to move a pinned piece.
#[test]
fn test_from_uci_illegal_move_leaves_king_in_check() {
    // Moving the pinned knight on c3 is illegal.
    let mut board = Board::from_fen("rnbqkbnr/pp1ppppp/8/2p5/8/2N5/PPPPPPPP/R1BQKBNR b KQkq - 1 2");
    // White bishop on b5 pins the black knight on c6 against the king on e8.
    // Moving the knight is illegal.
    board.stm = Side::Black; // Set to black's turn
    let result = Move::from_uci(&board, "c6e5");
    assert!(result.is_err(), "Should not allow moving a pinned piece");
}

#[test]
fn test_from_uci_invalid_format() {
    let board = Board::new();
    assert!(Move::from_uci(&board, "e2e4e5").is_err()); // Too long
    assert!(Move::from_uci(&board, "e2").is_err()); // Too short
    assert!(Move::from_uci(&board, "e2e4q").is_err()); // Valid length but not a promotion
}

#[test]
fn test_from_san_simple_pawn_move() {
    let board = Board::new();
    let mov = Move::from_san(&board, "e4").unwrap();
    assert_eq!(mov.uci(), "e2e4");
}

#[test]
fn test_from_san_knight_move() {
    let board = Board::new();
    let mov = Move::from_san(&board, "Nf3").unwrap();
    assert_eq!(mov.uci(), "g1f3");
}

#[test]
fn test_from_san_capture() {
    let board = Board::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1");
    let mov = Move::from_san(&board, "exd5").unwrap();
    assert_eq!(mov.uci(), "e4d5");
}

#[test]
fn test_from_san_promotion_with_check() {
    let board = Board::from_fen("r3k2r/pP1p1ppp/8/8/8/8/1P1P1P1P/R3K2R w KQkq - 0 1");
    // b7xa8=Q+
    let mov = Move::from_san(&board, "bxa8=Q+").unwrap();
    assert_eq!(mov.uci(), "b7a8q");
    assert_eq!(mov.flags(), Move::PROMO_QC);
}

#[test]
fn test_from_san_castling() {
    let board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
    let mov_ks = Move::from_san(&board, "O-O").unwrap();
    assert_eq!(mov_ks.uci(), "e1g1");

    let mov_qs = Move::from_san(&board, "O-O-O").unwrap();
    assert_eq!(mov_qs.uci(), "e1c1");
}

#[test]
fn test_from_san_disambiguation_file() {
    // Two rooks on the 1st rank can move to d1. "Rfd1" specifies the one from f1.
    let board = Board::from_fen("8/8/8/8/8/8/7K/R6R w - - 0 1");
    println!("{board}");
    let mov = Move::from_san(&board, "Rhd1").unwrap(); // Let's use Rhd1 to move the h-rook
    assert_eq!(mov.uci(), "h1d1");

    let mov2 = Move::from_san(&board, "Rad1").unwrap(); // And Rad1 for the a-rook
    assert_eq!(mov2.uci(), "a1d1");
}

#[test]
fn test_from_san_disambiguation_rank() {
    // Two rooks on the a-file can move to a4. "R1a4" specifies the one from a1.
    // The path for a1->a4 must be clear.
    let board = Board::from_fen("R3k3/8/8/8/8/8/8/R3K3 w Q - 0 1");
    let mov = Move::from_san(&board, "R1a4").unwrap();
    assert_eq!(mov.uci(), "a1a4");

    let mov2 = Move::from_san(&board, "R8a5").unwrap(); // And R8a5 for the other rook
    assert_eq!(mov2.uci(), "a8a5");
}

#[test]
fn test_from_san_illegal_move() {
    let board = Board::new();
    let result = Move::from_san(&board, "e5"); // Illegal pawn move
    assert!(result.is_err());
}

#[test]
fn test_rook_mask_from_center() {
    // Rook on d4
    let from = "d4".parse::<Square>().unwrap().index();
    let mask = MOVE_TABLES.generate_sliding_attack_mask(from, true);

    let expected_north = bb_from_squares(&["d5", "d6", "d7"]);
    let expected_south = bb_from_squares(&["d3", "d2"]);
    let expected_east = bb_from_squares(&["e4", "f4", "g4"]);
    let expected_west = bb_from_squares(&["c4", "b4"]);

    let expected = expected_north | expected_south | expected_east | expected_west;

    assert_eq!(
        mask,
        expected,
        "\nExpected:\n{}\nGot:\n{}",
        expected.print_bitboard(),
        mask.print_bitboard()
    );
}

#[test]
fn test_rook_mask_from_corner_a1() {
    // Rook on a1
    let from = "a1".parse::<Square>().unwrap().index();
    let mask = MOVE_TABLES.generate_sliding_attack_mask(from, true);

    let expected_north = bb_from_squares(&["a2", "a3", "a4", "a5", "a6", "a7"]);
    let expected_east = bb_from_squares(&["b1", "c1", "d1", "e1", "f1", "g1"]);

    let expected = expected_north | expected_east;

    assert_eq!(
        mask,
        expected,
        "\nExpected:\n{}\nGot:\n{}",
        expected.print_bitboard(),
        mask.print_bitboard()
    );
}

#[test]
fn test_rook_mask_from_edge_h4() {
    // Rook on h4
    let from = "h4".parse::<Square>().unwrap().index();
    let mask = MOVE_TABLES.generate_sliding_attack_mask(from, true);

    let expected_north = bb_from_squares(&["h5", "h6", "h7"]);
    let expected_south = bb_from_squares(&["h3", "h2"]);
    let expected_west = bb_from_squares(&["g4", "f4", "e4", "d4", "c4", "b4"]);

    let expected = expected_north | expected_south | expected_west;

    assert_eq!(
        mask,
        expected,
        "\nExpected:\n{}\nGot:\n{}",
        expected.print_bitboard(),
        mask.print_bitboard()
    );
}

#[test]
fn test_bishop_mask_from_center() {
    // Bishop on d4
    let from = "d4".parse::<Square>().unwrap().index();
    let mask = MOVE_TABLES.generate_sliding_attack_mask(from, false);

    let expected_ne = bb_from_squares(&["e5", "f6", "g7"]);
    let expected_se = bb_from_squares(&["e3", "f2"]);
    let expected_sw = bb_from_squares(&["c3", "b2"]);
    let expected_nw = bb_from_squares(&["c5", "b6"]);

    let expected = expected_ne | expected_se | expected_sw | expected_nw;

    assert_eq!(
        mask,
        expected,
        "\nExpected:\n{}\nGot:\n{}",
        expected.print_bitboard(),
        mask.print_bitboard()
    );
}

#[test]
fn test_bishop_mask_from_corner_a1() {
    // Bishop on a1
    let from = "a1".parse::<Square>().unwrap().index();
    let mask = MOVE_TABLES.generate_sliding_attack_mask(from, false);

    let expected_ne = bb_from_squares(&["b2", "c3", "d4", "e5", "f6", "g7"]);

    let expected = expected_ne;

    assert_eq!(
        mask,
        expected,
        "\nExpected:\n{}\nGot:\n{}",
        expected.print_bitboard(),
        mask.print_bitboard()
    );
}

#[test]
fn test_bishop_mask_from_edge_b1() {
    // Bishop on b1
    let from = "b1".parse::<Square>().unwrap().index();
    let mask = MOVE_TABLES.generate_sliding_attack_mask(from, false);

    let expected_ne = bb_from_squares(&["c2", "d3", "e4", "f5", "g6"]);
    // No other rays have inner squares

    let expected = expected_ne;

    assert_eq!(
        mask,
        expected,
        "\nExpected:\n{}\nGot:\n{}",
        expected.print_bitboard(),
        mask.print_bitboard()
    );
}
// verify symmetry of make_move and unmake_move
fn test_make_unmake_symmetry(fen: &str) {
    init();

    let mut board = Board::from_fen(fen);
    let original_board = board;

    let mut legal_moves = MoveBuffer::new();
    board.generate_legal_moves(&mut legal_moves, false);

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
    let mut legal_moves = MoveBuffer::new();
    board.generate_legal_moves(&mut legal_moves, false);
    let mut our_moves: Vec<String> = legal_moves.into_iter().map(|m| m.uci()).collect();
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
