//! Stateless Move Generation
//!
//! This module provides stateless move generation functions for chess pieces.
//! These functions generate pseudo-legal moves without maintaining internal state,
//! making them suitable for high-performance move generation in search algorithms.
//!
//! # Usage Example
//!
//! ```rust
//! use eschec::moves::move_gen::*;
//! use eschec::{Board, CastlingRights, Square, moves::move_info::Move};
//!
//! let board = Board::new();
//! let mut moves = Vec::new();
//!
//! // Generate all moves for white
//! let mut moves = Vec::new()
//! generate_all_moves(
//!     &board.positions,
//!     Side::White,
//!     board.castling_rights,
//!     board.enpassant_square,
//!     &mut moves
//! );
//!
//! // Or generate moves for specific piece types
//! let mut pawn_moves = Vec::new();
//! generate_piece_moves(
//!     Piece::Pawn,
//!     &board.positions,
//!     Side::White,
//!     board.castling_rights,
//!     board.enpassant_square,
//!     &mut pawn_moves
//! );
//! ```
//!
//! # Special Moves Handled
//!
//! - **Pawn double push**: Automatically generated for pawns on starting ranks
//! - **En passant**: Handled in `gen_pawn_moves_with_ep` when en passant square is provided
//! - **Promotions**: All promotion types (Q, R, B, N) generated for pawns reaching last rank
//! - **Castling**: Handled in `gen_king_moves_with_castling` when castling rights allow
//!
//! # Move Types Generated
//!
//! These functions generate **pseudo-legal** moves, meaning they don't check if the king
//! would be in check after the move. Legal move validation should be done at a higher level.

use crate::{Board, BoardState, CastlingRights, Piece, Side, board::components::Square};

use super::{move_info::Move, precomputed::MOVE_TABLES};

/// Generate all pseudo-legal moves for a side and return them as a new Vec
pub fn gen_all_moves_vec(board: &Board) -> Vec<Move> {
    let mut moves = Vec::new();

    generate_all_moves(
        &board.positions,
        board.stm,
        board.castling_rights,
        board.enpassant_square,
        &mut moves,
    );

    moves
}

/// Generate all pseudo-legal moves for a piece type and return them as a new Vec
pub fn generate_piece_moves_vec(
    piece: Piece,
    state: &BoardState,
    side: Side,
    castling_rights: CastlingRights,
    en_passant_square: Option<Square>,
) -> Vec<Move> {
    let mut moves = Vec::new();
    generate_piece_moves(
        piece,
        state,
        side,
        castling_rights,
        en_passant_square,
        &mut moves,
    );
    moves
}

/// Generate all pseudo-legal moves for a piece type at a specific square
/// and return them as a new Vec.
/// This is a convenience wrapper that filters out only moves from `from_square`.
pub fn generate_moves_from_square(
    piece: Piece,
    from_square: Square,
    state: &BoardState,
    side: Side,
    castling_rights: CastlingRights,
    en_passant_square: Option<Square>,
) -> Vec<Move> {
    let mut moves = Vec::new();
    generate_piece_moves(
        piece,
        state,
        side,
        castling_rights,
        en_passant_square,
        &mut moves,
    );
    moves
        .into_iter()
        .filter(|m| m.from_idx() as usize == from_square.index())
        .collect()
}

/// Comprehensive move generation function that handles all piece types and special moves
///
/// This function generates all pseudo-legal moves for the given side, including:
/// - Regular piece moves (pawns, knights, bishops, rooks, queens, kings)
/// - Special pawn moves (double push, en passant, promotions)
/// - Castling moves (kingside and queenside)
///
/// Note: These are pseudo-legal moves - they don't check if the king would be in check
/// after the move. That validation should be done at a higher level.
///
/// # Arguments
/// * `state` - Current board state with piece positions
/// * `side` - Side to generate moves for (White or Black)
/// * `castling_rights` - Current castling rights for both sides
/// * `en_passant_square` - Current en passant target square, if any
/// * `move_list` - Vector to append generated moves to
pub fn generate_all_moves(
    state: &BoardState,
    side: Side,
    castling_rights: CastlingRights,
    en_passant_square: Option<Square>,
    move_list: &mut Vec<Move>,
) {
    // Generate moves for all piece types
    gen_pawn_moves_with_ep(state, side, en_passant_square, move_list);
    gen_knight_moves(state, side, move_list);
    gen_bishop_moves(state, side, move_list);
    gen_rook_moves(state, side, move_list);
    gen_queen_moves(state, side, move_list);
    gen_king_moves_with_castling(state, side, castling_rights, move_list);
}

/// Generate all pseudo-legal moves for a specific piece type
///
/// This is useful when you only want to generate moves for one type of piece,
/// for example during move ordering or selective search.
///
/// # Arguments
/// * `piece` - The piece type to generate moves for
/// * `state` - Current board state with piece positions
/// * `side` - Side to generate moves for (White or Black)
/// * `castling_rights` - Current castling rights (only used for kings)
/// * `en_passant_square` - Current en passant target square (only used for pawns)
/// * `move_list` - Vector to append generated moves to
pub fn generate_piece_moves(
    piece: Piece,
    state: &BoardState,
    side: Side,
    castling_rights: CastlingRights,
    en_passant_square: Option<Square>,
    move_list: &mut Vec<Move>,
) {
    match piece {
        Piece::Pawn => gen_pawn_moves_with_ep(state, side, en_passant_square, move_list),
        Piece::Knight => gen_knight_moves(state, side, move_list),
        Piece::Bishop => gen_bishop_moves(state, side, move_list),
        Piece::Rook => gen_rook_moves(state, side, move_list),
        Piece::Queen => gen_queen_moves(state, side, move_list),
        Piece::King => gen_king_moves_with_castling(state, side, castling_rights, move_list),
    }
}

/// Generate knight moves
pub fn gen_knight_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
    let knights = state.get_piece_bb(side, Piece::Knight);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    let mut knights_bb = *knights;
    while let Some(from_sq) = knights_bb.pop_lsb() {
        let mut attacks = MOVE_TABLES.knight_moves[from_sq as usize] & !ally_pieces;
        while let Some(to_sq) = attacks.pop_lsb() {
            let is_capture = enemy_pieces.contains_square(to_sq as usize);
            let flag = if is_capture {
                Move::CAPTURE
            } else {
                Move::QUIET
            };
            move_list.push(Move::new(from_sq as u8, to_sq as u8, flag));
        }
    }
}

/// Generate king moves
pub fn gen_king_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
    let king = state.get_piece_bb(side, Piece::King);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    let mut king_bb = *king;
    while let Some(from_sq) = king_bb.pop_lsb() {
        let mut moves = MOVE_TABLES.king_moves[from_sq as usize] & !ally_pieces;
        while let Some(to_sq) = moves.pop_lsb() {
            let is_capture = enemy_pieces.contains_square(to_sq as usize);
            let flag = if is_capture {
                Move::CAPTURE
            } else {
                Move::QUIET
            };
            move_list.push(Move::new(from_sq as u8, to_sq as u8, flag));
        }
    }
}

/// Generate bishop moves
pub fn gen_bishop_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
    let bishops = state.get_piece_bb(side, Piece::Bishop);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    let mut bishops_bb = *bishops;
    while let Some(from_sq) = bishops_bb.pop_lsb() {
        let attacks = MOVE_TABLES.get_bishop_moves(from_sq as usize, *ally_pieces, *enemy_pieces);
        let mut attack_bb = attacks;
        while let Some(to_sq) = attack_bb.pop_lsb() {
            let is_capture = enemy_pieces.contains_square(to_sq as usize);
            let flag = if is_capture {
                Move::CAPTURE
            } else {
                Move::QUIET
            };
            move_list.push(Move::new(from_sq as u8, to_sq as u8, flag));
        }
    }
}

/// Generate queen moves
pub fn gen_queen_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
    let queens = state.get_piece_bb(side, Piece::Queen);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    let mut queens_bb = *queens;
    while let Some(from_sq) = queens_bb.pop_lsb() {
        let attacks = MOVE_TABLES.get_queen_moves(from_sq as usize, *ally_pieces, *enemy_pieces);
        let mut attack_bb = attacks;
        while let Some(to_sq) = attack_bb.pop_lsb() {
            let is_capture = enemy_pieces.contains_square(to_sq as usize);
            let flag = if is_capture {
                Move::CAPTURE
            } else {
                Move::QUIET
            };
            move_list.push(Move::new(from_sq as u8, to_sq as u8, flag));
        }
    }
}

/// Generate pawn moves
pub fn gen_rook_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
    let rooks = state.get_piece_bb(side, Piece::Rook);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    let mut rooks_bb = *rooks;
    while let Some(from_sq) = rooks_bb.pop_lsb() {
        let attacks = MOVE_TABLES.get_rook_moves(from_sq as usize, *ally_pieces, *enemy_pieces);
        let mut attack_bb = attacks;
        while let Some(to_sq) = attack_bb.pop_lsb() {
            let is_capture = enemy_pieces.contains_square(to_sq as usize);
            let flag = if is_capture {
                Move::CAPTURE
            } else {
                Move::QUIET
            };
            move_list.push(Move::new(from_sq as u8, to_sq as u8, flag));
        }
    }
}

/// Generate pawn moves
pub fn gen_pawn_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
    let pawns = state.get_piece_bb(side, Piece::Pawn);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    let mut pawns_bb = *pawns;
    while let Some(from_sq) = pawns_bb.pop_lsb() {
        let from = from_sq as usize;

        // Use precomputed helper for all legal pushes (single and double)
        let pushes = MOVE_TABLES.get_pawn_pushes(from, side, *ally_pieces, *enemy_pieces);

        let mut push_bb = pushes;
        while let Some(to_sq) = push_bb.pop_lsb() {
            let to_rank = to_sq as usize / 8;
            let is_promotion = match side {
                Side::White => to_rank == 7,
                Side::Black => to_rank == 0,
            };
            let is_double = match side {
                Side::White => (to_sq as usize) == from + 16,
                Side::Black => (to_sq as usize) == from - 16,
            };
            if is_promotion {
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::PROMO_Q));
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::PROMO_R));
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::PROMO_B));
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::PROMO_N));
            } else if is_double {
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::DOUBLE_PAWN));
            } else {
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::QUIET));
            }
        }

        // Captures (same as before)
        let attacks = MOVE_TABLES.get_pawn_attacks(from, side);
        let mut attack_bb = attacks & *enemy_pieces;
        while let Some(to_sq) = attack_bb.pop_lsb() {
            let to_rank = to_sq as usize / 8;
            let is_promotion = match side {
                Side::White => to_rank == 7,
                Side::Black => to_rank == 0,
            };
            if is_promotion {
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::PROMO_QC));
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::PROMO_RC));
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::PROMO_BC));
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::PROMO_NC));
            } else {
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::CAPTURE));
            }
        }
    }
}

/// Generate pawn moves with en passant support
pub fn gen_pawn_moves_with_ep(
    state: &BoardState,
    side: Side,
    en_passant_square: Option<Square>,
    move_list: &mut Vec<Move>,
) {
    // First generate all regular pawn moves
    gen_pawn_moves(state, side, move_list);

    // Then handle en passant if available
    if let Some(ep_square) = en_passant_square {
        let pawns = state.get_piece_bb(side, Piece::Pawn);
        let mut pawns_bb = *pawns;

        while let Some(from_sq) = pawns_bb.pop_lsb() {
            let from_sq_usize = from_sq as usize;
            let ep_sq_usize = ep_square.index();

            // Check if this pawn can capture en passant
            let attacks = MOVE_TABLES.get_pawn_attacks(from_sq_usize, side);
            if attacks.contains_square(ep_sq_usize) {
                move_list.push(Move::new(
                    from_sq as u8,
                    ep_sq_usize as u8,
                    Move::EN_PASSANT,
                ));
            }
        }
    }
}

/// Generate king moves with castling support
pub fn gen_king_moves_with_castling(
    state: &BoardState,
    side: Side,
    castling_rights: CastlingRights,
    move_list: &mut Vec<Move>,
) {
    // First generate all regular king moves
    gen_king_moves(state, side, move_list);

    // Then handle castling
    let king_bb = state.get_piece_bb(side, Piece::King);
    if let Some(king_pos) = king_bb.lsb() {
        let king_sq = king_pos as usize;
        let all_pieces = *state.get_side_bb(side) | *state.get_side_bb(side.flip());

        match side {
            Side::White => {
                // White king should be on e1 (square 4) for castling
                if king_sq == 4 {
                    // Kingside castling (e1-g1)
                    if castling_rights.allows(CastlingRights(CastlingRights::WHITE_00)) {
                        // Check if f1 and g1 are empty
                        if !all_pieces.contains_square(5) && !all_pieces.contains_square(6) {
                            move_list.push(Move::new(4, 6, Move::KING_CASTLE));
                        }
                    }

                    // Queenside castling (e1-c1)
                    if castling_rights.allows(CastlingRights(CastlingRights::WHITE_000)) {
                        // Check if b1, c1, and d1 are empty
                        if !all_pieces.contains_square(1)
                            && !all_pieces.contains_square(2)
                            && !all_pieces.contains_square(3)
                        {
                            move_list.push(Move::new(4, 2, Move::QUEEN_CASTLE));
                        }
                    }
                }
            }
            Side::Black => {
                // Black king should be on e8 (square 60) for castling
                if king_sq == 60 {
                    // Kingside castling (e8-g8)
                    if castling_rights.allows(CastlingRights(CastlingRights::BLACK_00)) {
                        // Check if f8 and g8 are empty
                        if !all_pieces.contains_square(61) && !all_pieces.contains_square(62) {
                            move_list.push(Move::new(60, 62, Move::KING_CASTLE));
                        }
                    }

                    // Queenside castling (e8-c8)
                    if castling_rights.allows(CastlingRights(CastlingRights::BLACK_000)) {
                        // Check if b8, c8, and d8 are empty
                        if !all_pieces.contains_square(57)
                            && !all_pieces.contains_square(58)
                            && !all_pieces.contains_square(59)
                        {
                            move_list.push(Move::new(60, 58, Move::QUEEN_CASTLE));
                        }
                    }
                }
            }
        }
    }
}

/// Check if a castling move would be pseudo-legal (doesn't check for check)
///
/// This only checks basic requirements:
/// - King is on the correct square
/// - Castling rights are available
/// - Path is clear of pieces
///
/// It does NOT check:
/// - If king is currently in check
/// - If king passes through check
/// - If king ends up in check
///
/// Those checks should be done at a higher level during legal move validation.
pub fn is_castling_pseudo_legal(
    state: &BoardState,
    side: Side,
    castling_rights: CastlingRights,
    is_kingside: bool,
) -> bool {
    let king_bb = state.get_piece_bb(side, Piece::King);
    if let Some(king_pos) = king_bb.lsb() {
        let king_sq = king_pos as usize;
        let all_pieces = *state.get_side_bb(side) | *state.get_side_bb(side.flip());

        match side {
            Side::White => {
                if king_sq != 4 {
                    return false;
                } // King must be on e1

                if is_kingside {
                    castling_rights.allows(CastlingRights(CastlingRights::WHITE_00))
                        && !all_pieces.contains_square(5) // f1 empty
                        && !all_pieces.contains_square(6) // g1 empty
                } else {
                    castling_rights.allows(CastlingRights(CastlingRights::WHITE_000))
                        && !all_pieces.contains_square(1) // b1 empty
                        && !all_pieces.contains_square(2) // c1 empty
                        && !all_pieces.contains_square(3) // d1 empty
                }
            }
            Side::Black => {
                if king_sq != 60 {
                    return false;
                } // King must be on e8

                if is_kingside {
                    castling_rights.allows(CastlingRights(CastlingRights::BLACK_00))
                        && !all_pieces.contains_square(61) // f8 empty
                        && !all_pieces.contains_square(62) // g8 empty
                } else {
                    castling_rights.allows(CastlingRights(CastlingRights::BLACK_000))
                        && !all_pieces.contains_square(57) // b8 empty
                        && !all_pieces.contains_square(58) // c8 empty
                        && !all_pieces.contains_square(59) // d8 empty
                }
            }
        }
    } else {
        false // No king found
    }
}
