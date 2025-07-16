//! Stateless Move Generation
//!
//! This module provides stateless move generation functions for chess pieces.
//! Legal Move Generation
//!
//! This module provides legal move generation functions for chess pieces.
//! It uses pre-calculated AttackData to efficiently determine legality
//! without making/unmaking moves on a board copy.

use crate::{
    BitBoard, Board, CastlingRights, Piece, Side,
    moves::{
        Direction, attack_data::calculate_attack_data, move_info::Move, precomputed::MOVE_TABLES,
    },
};

pub fn generate_legal_moves(board: &Board, moves: &mut Vec<Move>) {
    let side = board.stm;
    let attack_data = calculate_attack_data(board, side);

    if attack_data.double_check {
        gen_king_moves(board, &attack_data, moves);
        return;
    }

    gen_king_moves(board, &attack_data, moves);
    gen_pawn_moves(board, &attack_data, moves);
    gen_knight_moves(board, &attack_data, moves);
    gen_sliding_moves(board, Piece::Bishop, &attack_data, moves);
    gen_sliding_moves(board, Piece::Rook, &attack_data, moves);
    gen_sliding_moves(board, Piece::Queen, &attack_data, moves);
}

fn gen_king_moves(
    board: &Board,
    attack_data: &super::attack_data::AttackData,
    moves: &mut Vec<Move>,
) {
    let side = board.stm;
    let from_sq = attack_data.king_sq;
    let king_moves = MOVE_TABLES.king_moves[from_sq];
    let friendly_pieces = board.positions.get_side_bb(side);

    let mut legal_targets = king_moves & !friendly_pieces;

    while let Some(to_sq) = legal_targets.pop_lsb() {
        if !attack_data.opp_attack_map.contains_square(to_sq as usize) {
            let flag = if board.positions.is_occupied(to_sq as usize) {
                Move::CAPTURE
            } else {
                Move::QUIET
            };
            moves.push(Move::new(from_sq as u8, to_sq as u8, flag));
        }
    }

    // Castling
    if !attack_data.in_check {
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

fn gen_sliding_moves(
    board: &Board,
    piece: Piece,
    attack_data: &super::attack_data::AttackData,
    moves: &mut Vec<Move>,
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

fn gen_knight_moves(
    board: &Board,
    attack_data: &super::attack_data::AttackData,
    moves: &mut Vec<Move>,
) {
    let side = board.stm;
    let friendly_pieces = board.positions.get_side_bb(side);
    let enemy_pieces = board.positions.get_side_bb(side.flip());
    let mut knights =
        *board.positions.get_piece_bb(side, Piece::Knight) & !attack_data.pin_ray_mask;

    while let Some(from_sq) = knights.pop_lsb() {
        let attacks = MOVE_TABLES.knight_moves[from_sq as usize] & !friendly_pieces;
        let mut legal_targets = attacks & attack_data.check_ray_mask;

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

fn gen_pawn_moves(
    board: &Board,
    attack_data: &super::attack_data::AttackData,
    moves: &mut Vec<Move>,
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
                        moves.push(Move::new(from_sq as u8, two_steps as u8, Move::DOUBLE_PAWN));
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
