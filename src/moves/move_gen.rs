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
    moves::attack_data::{AttackData, calculate_attack_data},
    prelude::*,
};

pub trait MoveGenType {
    /// If true, quiet moves are filtered out
    const FORCING_ONLY: bool;
    /// If true, Only return captures/promotions
    const CAPTURES_ONLY: bool;
}

pub struct AllMoves;
impl MoveGenType for AllMoves {
    const FORCING_ONLY: bool = false;
    const CAPTURES_ONLY: bool = false;
}

pub struct ForcingMoves;
impl MoveGenType for ForcingMoves {
    const FORCING_ONLY: bool = true;
    const CAPTURES_ONLY: bool = false;
}

pub struct CapturesOnly;
impl MoveGenType for CapturesOnly {
    const FORCING_ONLY: bool = true;
    const CAPTURES_ONLY: bool = true;
}

// ===================================================================
//                      LEGAL MOVE GENERATION
// ===================================================================

/// Generates all strictly legal moves for the current side to move.
/// It accounts for checks, pins, and all special move rules.
pub fn generate_legal_moves<T: MoveGenType>(board: &Board, moves: &mut MoveBuffer) {
    let side = board.stm;
    let attack_data = calculate_attack_data(board, side);

    if attack_data.double_check {
        gen_legal_king_moves::<T>(board, &attack_data, moves);
        return;
    }

    gen_legal_king_moves::<T>(board, &attack_data, moves);
    gen_legal_pawn_moves::<T>(board, &attack_data, moves);
    gen_legal_knight_moves::<T>(board, &attack_data, moves);
    gen_legal_sliding_moves::<T>(board, Piece::Bishop, &attack_data, moves);
    gen_legal_sliding_moves::<T>(board, Piece::Rook, &attack_data, moves);
    gen_legal_sliding_moves::<T>(board, Piece::Queen, &attack_data, moves);
}

pub fn get_attackers_to(board: &Board, square: Square, side: Side, occupied: BitBoard) -> BitBoard {
    let sq_idx = square.index();
    let opponent = side.flip();
    let mut attackers = BitBoard::default();

    let pawn_attacks = MOVE_TABLES.get_pawn_attacks(sq_idx, opponent);
    attackers |= pawn_attacks & *board.positions.get_piece_bb(side, Piece::Pawn) & occupied;

    attackers |= MOVE_TABLES.knight_moves[sq_idx]
        & *board.positions.get_piece_bb(side, Piece::Knight)
        & occupied;

    attackers |= MOVE_TABLES.king_moves[sq_idx]
        & *board.positions.get_piece_bb(side, Piece::King)
        & occupied;

    let bishops_queens = board.positions.get_diag_sliders_bb(side);
    let rooks_queens = board.positions.get_ortho_sliders_bb(side);

    let bishop_attacks = MOVE_TABLES.get_bishop_attacks_generic(sq_idx, occupied);
    attackers |= bishop_attacks & bishops_queens & occupied;

    let rook_attacks = MOVE_TABLES.get_rook_attacks_generic(sq_idx, occupied);
    attackers |= rook_attacks & rooks_queens & occupied;

    attackers
}

fn gen_legal_king_moves<T: MoveGenType>(
    board: &Board,
    attack_data: &AttackData,
    moves: &mut MoveBuffer,
) {
    let side = board.stm;
    let from_sq = attack_data.king_sq;
    let king_moves = MOVE_TABLES.king_moves[from_sq];
    let friendly_pieces = board.positions.get_side_bb(side);

    let mut legal_targets = king_moves & !friendly_pieces;

    while legal_targets.any() {
        let to_sq = legal_targets.pop_lsb();
        if !attack_data.opp_attack_map.contains_square(to_sq as usize) {
            let is_capture = board.positions.is_occupied(to_sq as usize);

            // OPTIM: if only captures are needed, and this isn;t one,
            // skip immediately to avoid creating Move struct and calling 'is_move_a_check'
            if T::CAPTURES_ONLY && !is_capture {
                continue;
            }
            if !T::FORCING_ONLY || is_capture {
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
    // Castling is never a capture / forcing, so can be skipped
    if !T::CAPTURES_ONLY && !T::FORCING_ONLY && !attack_data.in_check {
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

fn gen_legal_sliding_moves<T: MoveGenType>(
    board: &Board,
    piece: Piece,
    attack_data: &AttackData,
    moves: &mut MoveBuffer,
) {
    let side = board.stm;
    let friendly_pieces = board.positions.get_side_bb(side);
    let enemy_pieces = board.positions.get_side_bb(side.flip());
    let mut piece_bb = *board.positions.get_piece_bb(side, piece);
    let opponent_king_sq = board
        .positions
        .get_piece_bb(board.stm.flip(), Piece::King)
        .lsb()
        .unwrap_or_default() as usize;

    while piece_bb.any() {
        let from_sq = piece_bb.pop_lsb();
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
        while legal_targets.any() {
            let to_sq = legal_targets.pop_lsb();
            let is_capture = enemy_pieces.contains_square(to_sq as usize);

            // OPTIM: if only captures are needed, and this isn;t one,
            // skip immediately to avoid creating Move struct and calling 'is_move_a_check'
            if T::CAPTURES_ONLY && !is_capture {
                continue;
            }

            let flag = if is_capture {
                Move::CAPTURE
            } else {
                Move::QUIET
            };
            let current_move = Move::new(from_sq as u8, to_sq as u8, flag);
            if !T::FORCING_ONLY
                || is_capture
                || (!T::CAPTURES_ONLY && is_move_a_check(board, current_move, opponent_king_sq))
            {
                moves.push(current_move);
            }
        }
    }
}

fn gen_legal_knight_moves<T: MoveGenType>(
    board: &Board,
    attack_data: &AttackData,
    moves: &mut MoveBuffer,
) {
    let side = board.stm;
    let friendly_pieces = board.positions.get_side_bb(side);
    let enemy_pieces = board.positions.get_side_bb(side.flip());
    let mut knights_bb =
        *board.positions.get_piece_bb(side, Piece::Knight) & !attack_data.pin_ray_mask;
    let opponent_king_sq = board
        .positions
        .get_piece_bb(board.stm.flip(), Piece::King)
        .lsb()
        .unwrap_or_default() as usize;

    while knights_bb.any() {
        let from_sq = knights_bb.pop_lsb();
        let attacks = MOVE_TABLES.knight_moves[from_sq as usize] & !friendly_pieces;
        let mut legal_targets = attacks & attack_data.check_ray_mask;

        while legal_targets.any() {
            let to_sq = legal_targets.pop_lsb();
            let is_capture = enemy_pieces.contains_square(to_sq as usize);

            // OPTIM: if only captures are needed, and this isn;t one,
            // skip immediately to avoid creating Move struct and calling 'is_move_a_check'
            if T::CAPTURES_ONLY && !is_capture {
                continue;
            }

            let flag = if is_capture {
                Move::CAPTURE
            } else {
                Move::QUIET
            };
            let current_move = Move::new(from_sq as u8, to_sq as u8, flag);
            if !T::FORCING_ONLY
                || is_capture
                || (!T::CAPTURES_ONLY && is_move_a_check(board, current_move, opponent_king_sq))
            {
                moves.push(current_move);
            }
        }
    }
}

fn gen_legal_pawn_moves<T: MoveGenType>(
    board: &Board,
    attack_data: &AttackData,
    moves: &mut MoveBuffer,
) {
    let side = board.stm;
    let pawns = board.positions.get_piece_bb(side, Piece::Pawn);
    let enemy_pieces = board.positions.get_side_bb(side.flip());
    let all_pieces =
        *board.positions.get_side_bb(Side::White) | *board.positions.get_side_bb(Side::Black);
    // TODO: Handle unwrap here
    let opponent_king_sq = board
        .positions
        .get_piece_bb(side.flip(), Piece::King)
        .lsb()
        .unwrap_or_default() as usize;

    let promo_rank = if side == Side::White { 7 } else { 0 };

    let mut pawns_bb = *pawns;
    while pawns_bb.any() {
        let from_sq = pawns_bb.pop_lsb();
        let from_sq_u = from_sq as usize;
        let is_pinned = attack_data.pin_ray_mask.contains_square(from_sq_u);
        let pin_dir = if is_pinned {
            Some(Direction::get_dir(attack_data.king_sq, from_sq_u))
        } else {
            None
        };

        // Pushes
        if !T::FORCING_ONLY {
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
        } else {
            // When forcing_only, need to check if a push can lead to a check (promotion)
            // Quiet moves are not forcing
            let push_dir = if side == Side::White {
                Direction::NORTH
            } else {
                Direction::SOUTH
            };
            if pin_dir.is_none() || pin_dir == Some(push_dir) {
                let one_step = (from_sq as i8 + push_dir) as usize;
                if !all_pieces.contains_square(one_step)
                    && attack_data.check_ray_mask.contains_square(one_step)
                {
                    // Only consider promotions, as they are the only pushes that can be forcing.
                    if one_step / 8 == promo_rank {
                        if T::CAPTURES_ONLY {
                            // in CapturesOnly, we want all promos for material change
                            add_promo_moves(from_sq as u8, one_step as u8, false, moves);
                        }
                        // Queen promo is most likey to result in checks
                        let promo_move = Move::new(from_sq as u8, one_step as u8, Move::PROMO_Q);
                        if is_move_a_check(board, promo_move, opponent_king_sq) {
                            add_promo_moves(from_sq as u8, one_step as u8, false, moves);
                        }
                    }
                }
            }
        }
        // Captures
        let attacks = MOVE_TABLES.get_pawn_attacks(from_sq_u, side);
        let mut capture_targets = attacks & *enemy_pieces;
        while capture_targets.any() {
            let to_sq = capture_targets.pop_lsb();
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
                let rooks_queens = board.positions.get_ortho_sliders_bb(side.flip());
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

fn add_promo_moves(from: u8, to: u8, is_capture: bool, moves: &mut MoveBuffer) {
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

/// Determines if a given pseudo-legal move delivers a check to the opponent.
/// Also handles discovered checks
fn is_move_a_check(board: &Board, mv: Move, opponent_king_sq: usize) -> bool {
    let from = mv.from_sq().index();
    let to = mv.to_sq().index();
    let piece = match board.get_piece_at(mv.from_sq()) {
        Some(p) => p,
        None => return false,
    };

    // Direct check from piece that is moved
    let piece_attacks_from_to_sq = get_piece_attacks(board, board.stm, piece, to);
    if piece_attacks_from_to_sq.contains_square(opponent_king_sq) {
        return true;
    }

    // Discovered check: see if moving this piece from 'from' square open up
    // an avenue of attack by other pieces.
    // NOTE: The moving piece is not a pinned piece, it is a friendly, so it cannot be pinned
    // Pinned pieces are those that block an enemy's attack
    let king_and_from_ray =
        MOVE_TABLES.get_ray(opponent_king_sq, Direction::get_dir(opponent_king_sq, from));
    // get_dir can return a (0), so this is to handle the case where the pieces are not
    // aligned
    if king_and_from_ray.contains_square(from) {
        let occupied = board.positions.get_occupied_bb() & !BitBoard(1 << from); // Remove 'from' sq

        let rooks_queens = board.positions.get_ortho_sliders_bb(board.stm);
        let bishops_queens = board.positions.get_diag_sliders_bb(board.stm);

        let magic_rook = MOVE_TABLES.get_rook_attacks_bb(opponent_king_sq, occupied);
        if (magic_rook & rooks_queens).any() {
            return true;
        }
        let magic_bishop = MOVE_TABLES.get_bishop_attacks_bb(opponent_king_sq, occupied);
        if (magic_bishop & bishops_queens).any() {
            return true;
        }
    }

    if mv.is_enpassant() {
        let captured_pawn_sq = if board.stm == Side::White {
            to - 8
        } else {
            to + 8
        };
        let occupied = (board.positions.get_occupied_bb()
            & !BitBoard(1 << from)
            & !BitBoard(1 << captured_pawn_sq))
            | BitBoard(1 << to);

        let rooks_queens = board.positions.get_ortho_sliders_bb(board.stm);
        let bishops_queens = board.positions.get_diag_sliders_bb(board.stm);

        let magic_rook = MOVE_TABLES.get_rook_attacks_bb(opponent_king_sq, occupied);
        if (magic_rook & rooks_queens).any() {
            return true;
        }

        let magic_bishop = MOVE_TABLES.get_bishop_attacks_bb(opponent_king_sq, occupied);
        if (magic_bishop & bishops_queens).any() {
            return true;
        }
    }
    false
}

fn get_piece_attacks(board: &Board, side: Side, piece: Piece, from: usize) -> BitBoard {
    match piece {
        Piece::Pawn => MOVE_TABLES.get_pawn_attacks(from, side),
        Piece::Knight => MOVE_TABLES.knight_moves[from],
        Piece::King => MOVE_TABLES.king_moves[from],
        Piece::Bishop => MOVE_TABLES.get_bishop_attacks_bb(from, board.positions.get_occupied_bb()),
        Piece::Rook => MOVE_TABLES.get_rook_attacks_bb(from, board.positions.get_occupied_bb()),
        Piece::Queen => {
            MOVE_TABLES.get_rook_attacks_bb(from, board.positions.get_occupied_bb())
                | MOVE_TABLES.get_bishop_attacks_bb(from, board.positions.get_occupied_bb())
        }
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
    move_list: &mut MoveBuffer,
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
    move_list: &mut MoveBuffer,
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
fn gen_knight_moves(state: &BoardState, side: Side, move_list: &mut MoveBuffer) {
    let mut knights_bb = *state.get_piece_bb(side, Piece::Knight);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    while knights_bb.any() {
        let from_sq = knights_bb.pop_lsb();
        let mut attacks = MOVE_TABLES.knight_moves[from_sq as usize] & !ally_pieces;
        while attacks.any() {
            let to_sq = attacks.pop_lsb();
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
fn gen_king_moves(state: &BoardState, side: Side, move_list: &mut MoveBuffer) {
    let mut king_bb = *state.get_piece_bb(side, Piece::King);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    while king_bb.any() {
        let from_sq = king_bb.pop_lsb();
        let mut moves = MOVE_TABLES.king_moves[from_sq as usize] & !ally_pieces;
        while moves.any() {
            let to_sq = moves.pop_lsb();
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
fn gen_bishop_moves(state: &BoardState, side: Side, move_list: &mut MoveBuffer) {
    let mut bishops_bb = *state.get_piece_bb(side, Piece::Bishop);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    while bishops_bb.any() {
        let from_sq = bishops_bb.pop_lsb();
        let attacks = MOVE_TABLES.get_bishop_moves(from_sq as usize, *ally_pieces, *enemy_pieces);
        let mut attack_bb = attacks;
        while attack_bb.any() {
            let to_sq = attack_bb.pop_lsb();
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
fn gen_queen_moves(state: &BoardState, side: Side, move_list: &mut MoveBuffer) {
    let mut queens_bb = *state.get_piece_bb(side, Piece::Queen);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    while queens_bb.any() {
        let from_sq = queens_bb.pop_lsb();
        let attacks = MOVE_TABLES.get_queen_moves(from_sq as usize, *ally_pieces, *enemy_pieces);
        let mut attack_bb = attacks;
        while attack_bb.any() {
            let to_sq = attack_bb.pop_lsb();
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
fn gen_rook_moves(state: &BoardState, side: Side, move_list: &mut MoveBuffer) {
    let mut rooks_bb = *state.get_piece_bb(side, Piece::Rook);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    while rooks_bb.any() {
        let from_sq = rooks_bb.pop_lsb();
        let attacks = MOVE_TABLES.get_rook_moves(from_sq as usize, *ally_pieces, *enemy_pieces);
        let mut attack_bb = attacks;
        while attack_bb.any() {
            let to_sq = attack_bb.pop_lsb();
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
fn gen_pawn_moves(state: &BoardState, side: Side, move_list: &mut MoveBuffer) {
    let mut pawns_bb = *state.get_piece_bb(side, Piece::Pawn);
    let ally_pieces = state.get_side_bb(side);
    let enemy_pieces = state.get_side_bb(side.flip());

    while pawns_bb.any() {
        let from_sq = pawns_bb.pop_lsb();
        let from = from_sq as usize;

        let mut push_bb = MOVE_TABLES.get_pawn_pushes(from, side, *ally_pieces, *enemy_pieces);

        while push_bb.any() {
            let to_sq = push_bb.pop_lsb();
            let to_rank = to_sq as usize / 8;
            let from_rank = from / 8;
            let is_promotion = match side {
                Side::White => to_rank == 7,
                Side::Black => to_rank == 0,
            };
            let is_double = match side {
                Side::White => from_rank == 1 && to_rank == 3,
                Side::Black => from_rank == 6 && to_rank == 4,
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
        while attack_bb.any() {
            let to_sq = attack_bb.pop_lsb();
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
    move_list: &mut MoveBuffer,
) {
    gen_pawn_moves(state, side, move_list);

    if let Some(ep_square) = en_passant_square {
        let mut pawns_bb = *state.get_piece_bb(side, Piece::Pawn);

        while pawns_bb.any() {
            let from_sq = pawns_bb.pop_lsb();
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
    move_list: &mut MoveBuffer,
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
