use crate::{
    BitBoard, Board, Piece, Side,
    moves::{Direction, precomputed::MOVE_TABLES},
};

#[derive(Debug, Clone, Default)]
pub struct AttackData {
    /// If one piece attacks king
    pub in_check: bool,
    /// If more than one piece attacks king
    pub double_check: bool,
    /// Bits set for pinned pieces (pieces that cannot move off their pin-ray)
    pub pin_ray_mask: BitBoard,
    /// Bits set for squares that can block a check or capture the checking piece.
    /// In a double check, this will be empty as only the king can move.
    /// If not in check, this will be all squares (all 1s).
    pub check_ray_mask: BitBoard,
    /// Bits set for the checking pieces themselves.
    pub checker_mask: BitBoard,
    /// All squares attacked by the opponent (for king move legality)
    pub opp_attack_map: BitBoard,
    /// The square of the friendly king
    pub king_sq: usize,
}

pub fn calculate_attack_data(board: &Board, side: Side) -> AttackData {
    let mut attack_data = AttackData {
        check_ray_mask: BitBoard(!0u64),
        ..Default::default()
    };

    let king_sq = match board.positions.get_piece_bb(side, Piece::King).lsb() {
        Some(sq) => sq as usize,
        None => return attack_data, // No king on board, no legality check
    };
    attack_data.king_sq = king_sq;

    let opponent = side.flip();
    let all_pieces =
        board.positions.get_side_bb(Side::White) | board.positions.get_side_bb(Side::Black);
    let friendly_pieces = board.positions.get_side_bb(side);

    // Sliding pieces
    let opp_rooks_queens = board.positions.get_orhto_sliders_bb(opponent);
    let opp_bishops_queens = board.positions.get_diag_sliders_bb(opponent);

    for &dir in &Direction::ALL {
        let is_forward_ray = dir > 0;
        let ray = MOVE_TABLES.get_ray(king_sq, dir);
        let blockers_on_ray = ray & all_pieces;

        if blockers_on_ray.any() {
            let first_blocker_sq =
                blockers_on_ray.get_closest_bit(is_forward_ray).unwrap() as usize;

            // If the first blocker is a friendly piece, it might be pinned.
            if friendly_pieces.contains_square(first_blocker_sq) {
                let blockers_behind_friendly = blockers_on_ray & !BitBoard(1 << first_blocker_sq);
                if let Some(potential_pinner_sq) =
                    blockers_behind_friendly.get_closest_bit(is_forward_ray)
                {
                    let potential_pinner_sq = potential_pinner_sq as usize;
                    let pinner_bb = BitBoard(1 << potential_pinner_sq);
                    let is_ortho_dir = matches!(
                        dir,
                        Direction::NORTH | Direction::SOUTH | Direction::EAST | Direction::WEST
                    );

                    if (is_ortho_dir && (pinner_bb & opp_rooks_queens).any())
                        || (!is_ortho_dir && (pinner_bb & opp_bishops_queens).any())
                    {
                        // It's a valid pin. The friendly piece at first_blocker_sq is pinned.
                        attack_data.pin_ray_mask.set(first_blocker_sq);
                    }
                }
            }
            // If the first blocker is an opponent piece, it's a check.
            else {
                let checker_bb = BitBoard(1 << first_blocker_sq);
                let is_ortho_dir = matches!(
                    dir,
                    Direction::NORTH | Direction::SOUTH | Direction::EAST | Direction::WEST
                );

                if (is_ortho_dir && (checker_bb & opp_rooks_queens).any())
                    || (!is_ortho_dir && (checker_bb & opp_bishops_queens).any())
                {
                    if attack_data.in_check {
                        attack_data.double_check = true;
                    }
                    attack_data.in_check = true;
                    attack_data.checker_mask |= checker_bb;
                    // The check ray is the line between the king and the checker, including the checker's square.
                    let check_ray = MOVE_TABLES.get_ray(king_sq, dir)
                        & MOVE_TABLES.get_ray(first_blocker_sq, -dir);
                    attack_data.check_ray_mask &= check_ray | checker_bb;
                }
            }
        }
    }

    // Knights, Pawn, King attacks
    let opp_knights = board.positions.get_piece_bb(opponent, Piece::Knight);
    let opp_pawns = board.positions.get_piece_bb(opponent, Piece::Pawn);
    let opp_king = board.positions.get_piece_bb(opponent, Piece::King);

    // Knight checks
    let knight_attacks = MOVE_TABLES.knight_moves[king_sq];
    let knight_checkers = knight_attacks & *opp_knights;
    if knight_checkers.any() {
        if attack_data.in_check {
            attack_data.double_check = true;
        }
        attack_data.in_check = true;
        attack_data.checker_mask |= knight_checkers;
        // for non-sliding checks, the only way to block is to capture the checker
        attack_data.check_ray_mask &= knight_checkers;
    }

    // Pawn checks
    let pawn_attacks = MOVE_TABLES.get_pawn_attacks(king_sq, side);
    let pawn_checkers = pawn_attacks & *opp_pawns;
    if pawn_checkers.any() {
        if attack_data.in_check {
            attack_data.double_check = true;
        }
        attack_data.in_check = true;
        attack_data.checker_mask |= pawn_checkers;
        attack_data.check_ray_mask &= pawn_checkers;
    }

    // Opponent attack map
    let all_pieces_no_king = all_pieces & !board.positions.get_piece_bb(side, Piece::King);

    if let Some(king_sq) = opp_king.lsb() {
        attack_data.opp_attack_map |= MOVE_TABLES.king_moves[king_sq as usize];
    }

    for pawn_sq in opp_pawns.iter_bits() {
        attack_data.opp_attack_map |= MOVE_TABLES.get_pawn_attacks(pawn_sq, opponent);
    }
    for knight_sq in opp_knights.iter_bits() {
        attack_data.opp_attack_map |= MOVE_TABLES.knight_moves[knight_sq];
    }
    // Bishop + Queen
    for diag_sq in board.positions.get_diag_sliders_bb(opponent).iter_bits() {
        attack_data.opp_attack_map |=
            MOVE_TABLES.get_bishop_attacks_generic(diag_sq, all_pieces_no_king);
    }
    // Rook + Queen
    for orhto_sq in board.positions.get_orhto_sliders_bb(opponent).iter_bits() {
        attack_data.opp_attack_map |=
            MOVE_TABLES.get_rook_attacks_generic(orhto_sq, all_pieces_no_king);
    }

    attack_data
}
