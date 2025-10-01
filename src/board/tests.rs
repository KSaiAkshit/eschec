use super::{Board, Move, Piece, Square};
use std::str::FromStr;

#[cfg(test)]
mod bitboard_tests {
    use crate::prelude::*;

    #[test]
    fn test_bb_lsb() {
        let bb = BitBoard(0b001000);
        let expected_lsb = 3;
        let lsb = bb.lsb();
        assert!(lsb.is_some(), "BitBoard::lsb() expected to return Some(x)");
        assert_eq!(
            lsb,
            Some(expected_lsb),
            "BitBoard::lsb() does not match expectations"
        );
    }
}

#[cfg(test)]
mod see_tests {
    use super::*;
    use crate::prelude::*;

    fn see(board: &Board, mv: Move) -> (i32, Vec<Piece>) {
        let from_sq = mv.from_sq();
        let to_sq = mv.to_sq();
        let mut side_to_move = board.stm;

        let mut capture_sequence = Vec::new();

        let mut occupied = board.positions.get_occupied_bb();
        let mut gain = [0; 32];
        let mut final_gain_idx = 0;

        let attacker_piece = match board.get_piece_at(from_sq) {
            Some(p) => p,
            None => unreachable!(
                "Move is supposed to be legal.
                There should be an attacker_piece at {from_sq}."
            ),
        };
        let victim_piece = if mv.is_enpassant() {
            Piece::Pawn
        } else {
            match board.get_piece_at(to_sq) {
                Some(p) => p,
                None => return (0, capture_sequence),
            }
        };

        gain[final_gain_idx] = victim_piece.victim_score();
        final_gain_idx += 1;
        gain[final_gain_idx] = attacker_piece.victim_score() - gain[final_gain_idx - 1];

        occupied.capture(from_sq.index());
        if mv.is_enpassant() {
            let captured_pawn_sq_idx = if side_to_move == Side::White {
                to_sq.get_neighbor(Direction::SOUTH).index()
            } else {
                to_sq.get_neighbor(Direction::NORTH).index()
            };
            occupied.capture(captured_pawn_sq_idx);
        } else {
            occupied.capture(to_sq.index());
        }
        // occupied.set(to_sq.index());

        capture_sequence.push(victim_piece);
        capture_sequence.push(attacker_piece);

        side_to_move = side_to_move.flip();

        // eprintln!("Victim scores: ");
        // for p in Piece::all_pieces() {
        //     eprintln!("{p}: {}", p.victim_score());
        // }
        // eprintln!(
        //     "attacker_piece: {attacker_piece}, victim_piece: {victim_piece}, gain[{}]: {:?}",
        //     final_gain_idx, gain[final_gain_idx]
        // );

        loop {
            final_gain_idx += 1;
            if final_gain_idx >= gain.len() {
                break;
            }
            let attackers_bb = move_gen::get_attackers_to(board, to_sq, side_to_move, occupied);

            let mut lva_piece = None;
            let mut lva_from_sq = None;

            for piece in Piece::all_pieces() {
                let lva_candidates =
                    attackers_bb & *board.positions.get_piece_bb(side_to_move, piece);
                if lva_candidates.any() {
                    lva_piece = Some(piece);
                    lva_from_sq = Square::new(lva_candidates.lsb().unwrap() as usize);
                    break;
                }
            }

            if let (Some(piece), Some(from)) = (lva_piece, lva_from_sq) {
                // eprintln!("Next attacker: {side_to_move} {piece} from {from}");
                gain[final_gain_idx] = piece.victim_score() - gain[final_gain_idx - 1];
                // eprintln!(
                //     "gain[{final_gain_idx}] = {} - gain[{}] = {} - {} = {}",
                //     piece.victim_score(),
                //     final_gain_idx - 1,
                //     piece.victim_score(),
                //     gain[final_gain_idx - 1],
                //     gain[final_gain_idx]
                // );

                occupied.capture(from.index());
                capture_sequence.push(piece);
                side_to_move = side_to_move.flip();
            } else {
                // eprintln!("No more attackers!");
                break;
            }
        }

        // dbg!(&gain[..=final_gain_idx]);
        final_gain_idx -= 1;
        while final_gain_idx > 1 {
            final_gain_idx -= 1;
            // eprintln!(
            //     "gain[{}] = gain[{}].min(-gain[{}]) = ({}).min({}) = {}",
            //     final_gain_idx - 1,
            //     final_gain_idx - 1,
            //     final_gain_idx,
            //     gain[final_gain_idx - 1],
            //     -gain[final_gain_idx],
            //     gain[final_gain_idx - 1].min(-gain[final_gain_idx])
            // );
            gain[final_gain_idx - 1] = gain[final_gain_idx - 1].min(-gain[final_gain_idx]);
        }

        (gain[0], capture_sequence)
    }

    #[test]
    fn test_fn_sync() {
        let fen = "1k1r4/1pp4p/p7/4p3/8/P5P1/1PP4P/2K1R3 w - - 0 1";
        let board = Board::from_fen(fen);
        let mv = Move::from_uci(&board, "e1e5").unwrap();
        let (free_see1, _seq) = see(&board, mv);
        let see1 = board.static_exchange_evaluation(mv);

        assert_eq!(
            see1, free_see1,
            "Free function and member function should return same val for pos #1"
        );
        let fen = "1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1";
        let board = Board::from_fen(fen);
        let mv = Move::from_uci(&board, "d3e5").unwrap();
        let (free_see2, _seq) = see(&board, mv);
        let see2 = board.static_exchange_evaluation(mv);
        assert_eq!(
            see2, free_see2,
            "Free function and member function should return same val for pos #2"
        );
    }

    #[test]
    fn test_short_see_sequence() {
        let fen = "1k1r4/1pp4p/p7/4p3/8/P5P1/1PP4P/2K1R3 w - - 0 1";
        let board = Board::from_fen(fen);
        let mv = Move::from_uci(&board, "e1e5").unwrap();
        let (see, seq) = see(&board, mv);

        dbg!(see);
        let expected_seq = vec![Piece::Pawn, Piece::Rook];
        assert_eq!(seq, expected_seq);
    }

    #[test]
    fn test_long_see_sequence() {
        use Piece::*;
        let fen = "1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1";
        let board = Board::from_fen(fen);
        let mv = Move::from_uci(&board, "d3e5").unwrap();
        let (see, seq) = see(&board, mv);

        dbg!(see);
        let expected_seq = vec![Pawn, Knight, Knight, Rook, Bishop, Queen, Queen];
        assert_eq!(seq, expected_seq);
    }
}

#[cfg(test)]
mod material_tests {

    use std::str::FromStr;

    use crate::prelude::*;

    use super::*;

    #[test]
    fn test_make_unmake_move() {
        init();
        let mut board = Board::new();
        let orig_board = board;

        let from = Square::from_str("e2").unwrap();
        let to = Square::from_str("e4").unwrap();
        let mov = Move::new(from.index() as u8, to.index() as u8, Move::QUIET);
        let orig_mat = board.material;

        let move_data = board.make_move(mov).unwrap();

        let moved_mat = board.material;

        assert_ne!(board, orig_board);
        assert_eq!(orig_mat, moved_mat);

        board.unmake_move(&move_data).unwrap();
        let restored_mat = board.material;

        assert_eq!(board, orig_board);
        assert_eq!(orig_mat, restored_mat);
    }

    #[test]
    fn test_make_unmake_capture() {
        init();
        let mut board =
            Board::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1");
        let orig_board = board;
        println!("{board}");

        let from = Square::from_str("e4").unwrap();
        let to = Square::from_str("d5").unwrap();
        let mov = Move::new(from.index() as u8, to.index() as u8, Move::CAPTURE);
        let orig_mat = board.material;

        let move_data = board.make_move(mov).unwrap();
        let moved_mat = board.material;

        assert_ne!(board, orig_board);
        assert_ne!(orig_mat, moved_mat);

        board.unmake_move(&move_data).unwrap();
        let restored_mat = board.material;

        println!("original board: \n{orig_board}");
        println!("unmade board: \n{board}");
        assert_eq!(board, orig_board);
        assert_eq!(orig_mat, restored_mat);
    }
    #[test]
    fn test_initial_material_balance() {
        let mut board = Board::new();
        board.recalculate_material();
        assert_eq!(
            board.material[Side::White.index()],
            Score::new(24039, 23868)
        );
        assert_eq!(
            board.material[Side::Black.index()],
            Score::new(24039, 23868)
        );
    }

    #[test]
    fn test_material_after_capture() {
        let mut board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPP1PP/RNBQKBNR w KQkq - 0 1");
        board.recalculate_material();
        // Standard position with a missing f2 pawn on white's side

        assert_eq!(
            board.material[Side::White.index()],
            Score::new(23957, 23774)
        );
        assert_eq!(
            board.material[Side::Black.index()],
            Score::new(24039, 23868)
        );
    }

    #[test]
    fn test_king_vs_king() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_king_and_bishop_vs_king() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KB2 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_king_and_knight_vs_king() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KN2 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_kings_and_same_colored_bishops() {
        // Bishops on the same color squares (both on light squares)
        let board = Board::from_fen("2b1k3/8/8/8/8/8/8/2B1K3 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_kings_and_different_colored_bishops() {
        // Bishops on different color squares (one on light, one on dark)
        let board = Board::from_fen("1b2k3/8/8/8/8/8/8/2B1K3 w - - 0 1");

        assert!(board.is_insufficient_material());
    }

    #[test]
    fn test_sufficient_material() {
        let board = Board::from_fen("4k3/8/8/8/8/8/4P3/4KB2 w - - 0 1");

        assert!(!board.is_insufficient_material());
    }

    #[test]
    fn test_two_knights_sufficient_material() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KNN1 w - - 0 1");

        assert!(!board.is_insufficient_material());
    }

    #[test]
    fn test_two_bishops_sufficient_material() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/3BKB2 w - - 0 1");

        assert!(!board.is_insufficient_material());
    }
}

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

    dbg!(move_data);
    println!(
        "Material after move -> W: {} B: {}",
        board.material[0], board.material[1]
    );
    println!(
        "Material after move -> W: {} B: {}",
        board.material[0], board.material[1]
    );

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
        board.material, original_board.material,
        "Scores are not restored perfectly"
    );
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
