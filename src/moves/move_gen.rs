//! Move Generation
//!
//! This module provides functions for generating both pseudo-legal and legal chess moves.
//!
//! # Pseudo-Legal vs. Legal Moves
//!
//! - **Pseudo-legal moves**: All moves a piece can make, including special moves like
//!   castling and en passant, without considering whether the king is left in check.
//!   These are generated quickly and are useful for tasks like mobility evaluation or
//!   the first stage of full legal move generation.
//!
//! - **Legal moves**: A subset of pseudo-legal moves that are guaranteed not to leave
//!   the king in check. This is what the engine's search and game logic uses. This
//!   generator uses pre-calculated `AttackData` to efficiently determine legality.

use crate::{
    BitBoard, Board, BoardState, CastlingRights, Piece, Side, Square,
    moves::{
        Direction,
        attack_data::{AttackData, calculate_attack_data},
        move_info::Move,
        precomputed::MOVE_TABLES,
    },
};

// ===================================================================
//                      LEGAL MOVE GENERATION
// ===================================================================

/// Generates all strictly legal moves for the current side to move.
/// It accounts for checks, pins, and all special move rules.
pub fn generate_legal_moves(board: &Board, moves: &mut Vec<Move>) {
    let side = board.stm;
    let attack_data = calculate_attack_data(board, side);

    if attack_data.double_check {
        gen_legal_king_moves(board, &attack_data, moves, false);
        return;
    }

    gen_legal_king_moves(board, &attack_data, moves, false);
    gen_legal_pawn_moves(board, &attack_data, moves, false);
    gen_legal_knight_moves(board, &attack_data, moves, false);
    gen_legal_sliding_moves(board, Piece::Bishop, &attack_data, moves, false);
    gen_legal_sliding_moves(board, Piece::Rook, &attack_data, moves, false);
    gen_legal_sliding_moves(board, Piece::Queen, &attack_data, moves, false);
}

pub fn generate_legal_captures(board: &Board, moves: &mut Vec<Move>) {
    let side = board.stm;
    let attack_data = calculate_attack_data(board, side);

    if attack_data.double_check {
        gen_legal_king_moves(board, &attack_data, moves, true);
    }

    gen_legal_pawn_moves(board, &attack_data, moves, true);
    gen_legal_knight_moves(board, &attack_data, moves, true);
    gen_legal_sliding_moves(board, Piece::Bishop, &attack_data, moves, true);
    gen_legal_sliding_moves(board, Piece::Rook, &attack_data, moves, true);
    gen_legal_sliding_moves(board, Piece::Queen, &attack_data, moves, true);
}

fn gen_legal_king_moves(
    board: &Board,
    attack_data: &AttackData,
    moves: &mut Vec<Move>,
    captures_only: bool,
) {
    let side = board.stm;
    let from_sq = attack_data.king_sq;
    let king_moves = MOVE_TABLES.king_moves[from_sq];
    let friendly_pieces = board.positions.get_side_bb(side);

    let mut legal_targets = king_moves & !friendly_pieces;

    while let Some(to_sq) = legal_targets.pop_lsb() {
        if !attack_data.opp_attack_map.contains_square(to_sq as usize) {
            let is_capture = board.positions.is_occupied(to_sq as usize);
            if !captures_only || is_capture {
                let flag = if is_capture {
                    Move::CAPTURE
                } else {
                    Move::QUIET
                };
                moves.push(Move::new(from_sq as u8, to_sq as u8, flag));
            };
        }
    }

    // Castling
    if !captures_only && !attack_data.in_check {
        let all_pieces =
            board.positions.get_side_bb(Side::White) | board.positions.get_side_bb(Side::Black);
        match side {
            Side::White => {
                // Kingside
                if board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::WHITE_00))
                    && !all_pieces.contains_square(5)
                    && !all_pieces.contains_square(6)
                    && !attack_data.opp_attack_map.contains_square(5)
                    && !attack_data.opp_attack_map.contains_square(6)
                {
                    moves.push(Move::new(4, 6, Move::KING_CASTLE));
                }
                // Queenside
                if board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::WHITE_000))
                    && !all_pieces.contains_square(1)
                    && !all_pieces.contains_square(2)
                    && !all_pieces.contains_square(3)
                    && !attack_data.opp_attack_map.contains_square(2)
                    && !attack_data.opp_attack_map.contains_square(3)
                {
                    moves.push(Move::new(4, 2, Move::QUEEN_CASTLE));
                }
            }
            Side::Black => {
                if board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::BLACK_00))
                    && !all_pieces.contains_square(61)
                    && !all_pieces.contains_square(62)
                    && !attack_data.opp_attack_map.contains_square(61)
                    && !attack_data.opp_attack_map.contains_square(62)
                {
                    moves.push(Move::new(60, 62, Move::KING_CASTLE));
                }
                // Queenside
                if board
                    .castling_rights
                    .allows(CastlingRights(CastlingRights::BLACK_000))
                    && !all_pieces.contains_square(57)
                    && !all_pieces.contains_square(58)
                    && !all_pieces.contains_square(59)
                    && !attack_data.opp_attack_map.contains_square(58)
                    && !attack_data.opp_attack_map.contains_square(59)
                {
                    moves.push(Move::new(60, 58, Move::QUEEN_CASTLE));
                }
            }
        }
    }
}

fn gen_legal_sliding_moves(
    board: &Board,
    piece: Piece,
    attack_data: &AttackData,
    moves: &mut Vec<Move>,
    captures_only: bool,
) {
    let side = board.stm;
    let friendly_pieces = board.positions.get_side_bb(side);
    let enemy_pieces = board.positions.get_side_bb(side.flip());
    let mut piece_bb = *board.positions.get_piece_bb(side, piece);

    while let Some(from_sq) = piece_bb.pop_lsb() {
        let is_pinned = attack_data.pin_ray_mask.contains_square(from_sq as usize);
        let mut move_mask = if is_pinned {
            MOVE_TABLES.get_ray(
                attack_data.king_sq,
                Direction::get_dir(attack_data.king_sq, from_sq as usize),
            )
        } else {
            BitBoard(!0u64)
        };
        move_mask &= attack_data.check_ray_mask;

        let attacks = match piece {
            Piece::Bishop => {
                MOVE_TABLES.get_bishop_moves(from_sq as usize, *friendly_pieces, *enemy_pieces)
            }
            Piece::Rook => {
                MOVE_TABLES.get_rook_moves(from_sq as usize, *friendly_pieces, *enemy_pieces)
            }
            Piece::Queen => {
                MOVE_TABLES.get_queen_moves(from_sq as usize, *friendly_pieces, *enemy_pieces)
            }
            _ => BitBoard(0),
        };

        let mut legal_targets = attacks & move_mask;
        if captures_only {
            legal_targets &= *enemy_pieces;
        }
        while let Some(to_sq) = legal_targets.pop_lsb() {
            let flag = if enemy_pieces.contains_square(to_sq as usize) {
                Move::CAPTURE
            } else {
                Move::QUIET
            };
            moves.push(Move::new(from_sq as u8, to_sq as u8, flag));
        }
    }
}

fn gen_legal_knight_moves(
    board: &Board,
    attack_data: &AttackData,
    moves: &mut Vec<Move>,
    captures_only: bool,
) {
    let side = board.stm;
    let friendly_pieces = board.positions.get_side_bb(side);
    let enemy_pieces = board.positions.get_side_bb(side.flip());
    let mut knights =
        *board.positions.get_piece_bb(side, Piece::Knight) & !attack_data.pin_ray_mask;

    while let Some(from_sq) = knights.pop_lsb() {
        let attacks = MOVE_TABLES.knight_moves[from_sq as usize] & !friendly_pieces;
        let mut legal_targets = attacks & attack_data.check_ray_mask;
        if captures_only {
            legal_targets &= *enemy_pieces;
        }

        while let Some(to_sq) = legal_targets.pop_lsb() {
            let flag = if enemy_pieces.contains_square(to_sq as usize) {
                Move::CAPTURE
            } else {
                Move::QUIET
            };
            moves.push(Move::new(from_sq as u8, to_sq as u8, flag));
        }
    }
}

fn gen_legal_pawn_moves(
    board: &Board,
    attack_data: &AttackData,
    moves: &mut Vec<Move>,
    captures_only: bool,
) {
    let side = board.stm;
    let pawns = board.positions.get_piece_bb(side, Piece::Pawn);
    let enemy_pieces = board.positions.get_side_bb(side.flip());
    let all_pieces =
        *board.positions.get_side_bb(Side::White) | *board.positions.get_side_bb(Side::Black);

    let promo_rank = if side == Side::White { 7 } else { 0 };

    let mut pawns_bb = *pawns;
    while let Some(from_sq) = pawns_bb.pop_lsb() {
        let from_sq_u = from_sq as usize;
        let is_pinned = attack_data.pin_ray_mask.contains_square(from_sq_u);
        let pin_dir = if is_pinned {
            Some(Direction::get_dir(attack_data.king_sq, from_sq_u))
        } else {
            None
        };

        // Pushes
        if !captures_only {
            let push_dir = if side == Side::White {
                Direction::NORTH
            } else {
                Direction::SOUTH
            };
            if pin_dir.is_none() || pin_dir == Some(push_dir) {
                let one_step = from_sq_u as i8 + push_dir;
                if !all_pieces.contains_square(one_step as usize) {
                    if attack_data
                        .check_ray_mask
                        .contains_square(one_step as usize)
                    {
                        if (one_step as usize) / 8 == promo_rank {
                            add_promo_moves(from_sq as u8, one_step as u8, false, moves);
                        } else {
                            moves.push(Move::new(from_sq as u8, one_step as u8, Move::QUIET));
                        }
                    }
                    // Double push
                    let start_rank = if side == Side::White { 1 } else { 6 };
                    if from_sq_u / 8 == start_rank {
                        let two_steps = from_sq_u as i8 + 2 * push_dir.value();
                        if !all_pieces.contains_square(two_steps as usize)
                            && attack_data
                                .check_ray_mask
                                .contains_square(two_steps as usize)
                        {
                            moves.push(Move::new(
                                from_sq as u8,
                                two_steps as u8,
                                Move::DOUBLE_PAWN,
                            ));
                        }
                    }
                }
            }
        }
        // Captures
        let attacks = MOVE_TABLES.get_pawn_attacks(from_sq_u, side);
        let mut capture_targets = attacks & *enemy_pieces;
        while let Some(to_sq) = capture_targets.pop_lsb() {
            let to_sq_u = to_sq as usize;
            let capture_dir = Direction::get_dir(from_sq_u, to_sq_u);
            if (pin_dir.is_none() || pin_dir == Some(capture_dir))
                && attack_data.check_ray_mask.contains_square(to_sq_u)
            {
                if to_sq_u / 8 == promo_rank {
                    add_promo_moves(from_sq as u8, to_sq as u8, true, moves);
                } else {
                    moves.push(Move::new(from_sq as u8, to_sq as u8, Move::CAPTURE));
                }
            }
        }

        // En Passant
        if let Some(ep_sq) = board.enpassant_square
            && (attacks & BitBoard(1 << ep_sq.index())).any()
        {
            let ep_dir = Direction::get_dir(from_sq_u, ep_sq.index());
            if pin_dir.is_none() || pin_dir == Some(ep_dir) {
                // En passant check is complex: need to see if removing both pawns opens a check.
                let captured_pawn_sq = if side == Side::White {
                    ep_sq.index() - 8
                } else {
                    ep_sq.index() + 8
                };
                let occupied_after_ep =
                    (all_pieces & !BitBoard(1 << from_sq_u) & !BitBoard(1 << captured_pawn_sq))
                        | BitBoard(1 << ep_sq.index());
                let king_sq = attack_data.king_sq;
                let rooks_queens = board.positions.get_orhto_sliders_bb(side.flip());
                let bishops_queens = board.positions.get_diag_sliders_bb(side.flip());

                let rook_attacks =
                    MOVE_TABLES.get_rook_moves(king_sq, BitBoard(0), occupied_after_ep);
                let bishop_attacks =
                    MOVE_TABLES.get_bishop_moves(king_sq, BitBoard(0), occupied_after_ep);

                if (rook_attacks & rooks_queens).is_empty()
                    && (bishop_attacks & bishops_queens).is_empty()
                {
                    moves.push(Move::new(
                        from_sq as u8,
                        ep_sq.index() as u8,
                        Move::EN_PASSANT,
                    ));
                }
            }
        }
    }
}

fn add_promo_moves(from: u8, to: u8, is_capture: bool, moves: &mut Vec<Move>) {
    if is_capture {
        moves.push(Move::new(from, to, Move::PROMO_QC));
        moves.push(Move::new(from, to, Move::PROMO_RC));
        moves.push(Move::new(from, to, Move::PROMO_BC));
        moves.push(Move::new(from, to, Move::PROMO_NC));
    } else {
        moves.push(Move::new(from, to, Move::PROMO_Q));
        moves.push(Move::new(from, to, Move::PROMO_R));
        moves.push(Move::new(from, to, Move::PROMO_B));
        moves.push(Move::new(from, to, Move::PROMO_N));
    }
}

// ===================================================================
//                  PSEUDO-LEGAL MOVE GENERATION
// ===================================================================

/// Comprehensive pseudo-legal move generation function.
///
/// This function generates all pseudo-legal moves for the given side, including:
/// - Regular piece moves (pawns, knights, bishops, rooks, queens, kings)
/// - Special pawn moves (double push, en passant, promotions)
/// - Castling moves (kingside and queenside)
pub fn generate_pseudo_legal_moves(
    state: &BoardState,
    side: Side,
    castling_rights: CastlingRights,
    en_passant_square: Option<Square>,
    move_list: &mut Vec<Move>,
) {
    gen_pawn_moves_with_ep(state, side, en_passant_square, move_list);
    gen_knight_moves(state, side, move_list);
    gen_bishop_moves(state, side, move_list);
    gen_rook_moves(state, side, move_list);
    gen_queen_moves(state, side, move_list);
    gen_king_moves_with_castling(state, side, castling_rights, move_list);
}

/// Generate pseudo-legal moves for a specific piece type.
pub fn generate_pseudo_legal_piece_moves(
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

/// Generate pseudo-legal knight moves.
fn gen_knight_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
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

/// Generate pseudo-legal king moves (without castling).
fn gen_king_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
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

/// Generate pseudo-legal bishop moves.
fn gen_bishop_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
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

/// Generate pseudo-legal queen moves.
fn gen_queen_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
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

/// Generate pseudo-legal rook moves.
fn gen_rook_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
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

/// Generate pseudo-legal pawn moves (without en passant).
fn gen_pawn_moves(state: &BoardState, side: Side, move_list: &mut Vec<Move>) {
    let pawns = state.get_piece_bb(side, Piece::Pawn);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    let mut pawns_bb = *pawns;
    while let Some(from_sq) = pawns_bb.pop_lsb() {
        let from = from_sq as usize;

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
                add_promo_moves(from_sq as u8, to_sq as u8, false, move_list);
            } else if is_double {
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::DOUBLE_PAWN));
            } else {
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::QUIET));
            }
        }

        let attacks = MOVE_TABLES.get_pawn_attacks(from, side);
        let mut attack_bb = attacks & *enemy_pieces;
        while let Some(to_sq) = attack_bb.pop_lsb() {
            let to_rank = to_sq as usize / 8;
            let is_promotion = match side {
                Side::White => to_rank == 7,
                Side::Black => to_rank == 0,
            };
            if is_promotion {
                add_promo_moves(from_sq as u8, to_sq as u8, true, move_list);
            } else {
                move_list.push(Move::new(from_sq as u8, to_sq as u8, Move::CAPTURE));
            }
        }
    }
}

/// Generate pseudo-legal pawn moves with en passant support.
fn gen_pawn_moves_with_ep(
    state: &BoardState,
    side: Side,
    en_passant_square: Option<Square>,
    move_list: &mut Vec<Move>,
) {
    gen_pawn_moves(state, side, move_list);

    if let Some(ep_square) = en_passant_square {
        let pawns = state.get_piece_bb(side, Piece::Pawn);
        let mut pawns_bb = *pawns;

        while let Some(from_sq) = pawns_bb.pop_lsb() {
            let attacks = MOVE_TABLES.get_pawn_attacks(from_sq as usize, side);
            if attacks.contains_square(ep_square.index()) {
                move_list.push(Move::new(
                    from_sq as u8,
                    ep_square.index() as u8,
                    Move::EN_PASSANT,
                ));
            }
        }
    }
}

/// Generate pseudo-legal king moves with castling support.
fn gen_king_moves_with_castling(
    state: &BoardState,
    side: Side,
    castling_rights: CastlingRights,
    move_list: &mut Vec<Move>,
) {
    gen_king_moves(state, side, move_list);

    let king_bb = state.get_piece_bb(side, Piece::King);
    if let Some(king_pos) = king_bb.lsb() {
        let king_sq = king_pos as usize;
        let all_pieces = *state.get_side_bb(side) | *state.get_side_bb(side.flip());

        match side {
            Side::White => {
                if king_sq == 4 {
                    if castling_rights.allows(CastlingRights(CastlingRights::WHITE_00))
                        && !all_pieces.contains_square(5)
                        && !all_pieces.contains_square(6)
                    {
                        move_list.push(Move::new(4, 6, Move::KING_CASTLE));
                    }
                    if castling_rights.allows(CastlingRights(CastlingRights::WHITE_000))
                        && !all_pieces.contains_square(1)
                        && !all_pieces.contains_square(2)
                        && !all_pieces.contains_square(3)
                    {
                        move_list.push(Move::new(4, 2, Move::QUEEN_CASTLE));
                    }
                }
            }
            Side::Black => {
                if king_sq == 60 {
                    if castling_rights.allows(CastlingRights(CastlingRights::BLACK_00))
                        && !all_pieces.contains_square(61)
                        && !all_pieces.contains_square(62)
                    {
                        move_list.push(Move::new(60, 62, Move::KING_CASTLE));
                    }
                    if castling_rights.allows(CastlingRights(CastlingRights::BLACK_000))
                        && !all_pieces.contains_square(57)
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
