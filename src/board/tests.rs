use super::{Board, Move, Piece, Square};
use std::str::FromStr;

/// A helper to initialize tracing for tests, making debug output visible.
/// Call this at the start of each test.
fn init_test_logging() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
}

#[test]
fn test_black_kingside_castle_moves_rook_correctly() {
    init_test_logging();
    // FEN where it's Black's turn and kingside castling is legal.
    let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 1 1";
    let mut board = Board::from_fen(fen);

    // The move for black kingside castling (e8g8).
    let castle_move = Move::new(60, 62, Move::KING_CASTLE);

    // Make the move.
    board.make_move(castle_move).unwrap();

    // 1. Assert the king moved correctly from e8 to g8.
    assert!(
        board
            .get_piece_at(Square::from_str("e8").unwrap())
            .is_none(),
        "King should have moved from e8"
    );
    assert_eq!(
        board.get_piece_at(Square::from_str("g8").unwrap()),
        Some(Piece::King),
        "King should be on g8"
    );

    // 2. Assert the rook moved correctly from h8 to f8.
    assert!(
        board
            .get_piece_at(Square::from_str("h8").unwrap())
            .is_none(),
        "Rook should have moved from h8"
    );
    assert_eq!(
        board.get_piece_at(Square::from_str("f8").unwrap()),
        Some(Piece::Rook),
        "Rook should be on f8"
    );

    // 3. Assert FEN representation is correct.
    let expected_fen_pieces = "r4rk1/pppppppp/8/8/8/8/PPPPPPPP/R3K2R";
    assert_eq!(board.positions.to_fen_pieces(), expected_fen_pieces);
}

#[test]
fn test_en_passant_make_unmake_symmetry() {
    init_test_logging();
    // FEN where White can capture en passant on d6.
    let fen = "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3";
    let mut board = Board::from_fen(fen);
    let original_board = board; // Save the original state.

    // The en passant move e5xd6.
    let ep_move = Move::new(
        Square::from_str("e5").unwrap().index() as u8,
        Square::from_str("d6").unwrap().index() as u8,
        Move::EN_PASSANT,
    );

    // Make the move and get the data to unmake it.
    let move_data = board.make_move(ep_move).unwrap();

    // Assert the captured black pawn on d5 is gone.
    assert!(
        board
            .get_piece_at(Square::from_str("d5").unwrap())
            .is_none(),
        "Black pawn on d5 should have been captured"
    );

    // Unmake the move.
    board.unmake_move(&move_data).unwrap();

    assert_eq!(
        board, original_board,
        "Board state was not perfectly restored after unmaking en passant move"
    );
}

// General Correctness and Regression Tests for the New `make_move`

#[test]
fn test_rook_capture_removes_opponent_castling_rights() {
    init_test_logging();
    // FEN where both sides can castle. It's White's turn.
    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let mut board = Board::from_fen(fen);

    // White moves Ra1xa8, capturing Black's rook.
    let capture_move = Move::new(
        Square::from_str("a1").unwrap().index() as u8,
        Square::from_str("a8").unwrap().index() as u8,
        Move::CAPTURE,
    );

    // Make the move.
    board.make_move(capture_move).unwrap();

    assert!(
        !board.castling_rights.can_castle(super::Side::Black, false),
        "Black should lose queenside castling rights after rook on a8 is captured"
    );
    assert!(
        board.castling_rights.can_castle(super::Side::Black, true),
        "Black should still have kingside castling rights"
    );
}

#[test]
fn test_promotion_to_queen_with_capture() {
    init_test_logging();
    // White pawn on b7 can capture on a8 and promote.
    let fen = "r3k2r/1Ppppppp/8/8/8/8/1P2PPPP/R3K2R w KQkq - 0 1";
    let mut board = Board::from_fen(fen);
    let original_board = board;

    // The move b7xa8=Q.
    let promo_capture = Move::new(
        Square::from_str("b7").unwrap().index() as u8,
        Square::from_str("a8").unwrap().index() as u8,
        Move::PROMO_QC, // Promote to Queen with Capture
    );

    let move_data = board.make_move(promo_capture).unwrap();

    // Assert the new piece is a White Queen on a8.
    assert_eq!(
        board.get_piece_at(Square::from_str("a8").unwrap()),
        Some(Piece::Queen)
    );
    assert_eq!(
        board
            .positions
            .get_piece_bb(super::Side::White, Piece::Queen)
            .pop_count(),
        1
    );
    assert_eq!(
        board
            .positions
            .get_piece_bb(super::Side::White, Piece::Pawn)
            .pop_count(),
        5
    );

    // Unmake the move and check for perfect restoration.
    board.unmake_move(&move_data).unwrap();
    assert_eq!(
        board, original_board,
        "Board not restored after unmaking promotion with capture"
    );
}
