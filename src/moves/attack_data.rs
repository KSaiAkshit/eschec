use crate::prelude::*;
use std::fmt::Debug;

#[derive(Clone, Default, PartialEq)]
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
    /// The square of the friendly king
    pub king_sq: usize,
}

impl Debug for AttackData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttackData")
            .field("in_check", &self.in_check)
            .field("double_check", &self.double_check)
            .field("pin_ray_mask", &self.pin_ray_mask.print_bitboard())
            .field("check_ray_mask", &self.check_ray_mask.print_bitboard())
            .field("checker_mask", &self.checker_mask.print_bitboard())
            .field("king_sq", &self.king_sq)
            .finish()
    }
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
    let occupied = board.positions.get_occupied_bb();
    let friendly_pieces = board.positions.get_side_bb(side);
    let _enemy_pieces = board.positions.get_side_bb(opponent);

    // Enemy Sliding pieces
    let opp_rooks_queens = board.positions.get_ortho_sliders_bb(opponent);
    let opp_bishops_queens = board.positions.get_diag_sliders_bb(opponent);

    let ortho_attacks = MOVE_TABLES.get_rook_attacks_bb(king_sq, occupied);
    let diag_attacks = MOVE_TABLES.get_bishop_attacks_bb(king_sq, occupied);

    let ortho_checkers = ortho_attacks & opp_rooks_queens;
    let diag_checkers = diag_attacks & opp_bishops_queens;
    let mut checkers = ortho_checkers | diag_checkers;

    let opp_knights = board.positions.get_piece_bb(opponent, Piece::Knight);
    let knight_attacks = MOVE_TABLES.knight_moves[king_sq];
    checkers |= knight_attacks & *opp_knights;

    let opp_pawns = board.positions.get_piece_bb(opponent, Piece::Pawn);
    let pawn_attacks = MOVE_TABLES.get_pawn_attacks(king_sq, side);
    checkers |= pawn_attacks & *opp_pawns;

    let checker_count = checkers.pop_count();
    if checker_count > 0 {
        attack_data.in_check = true;
        attack_data.checker_mask = checkers;

        if checker_count > 1 {
            attack_data.double_check = true;
            attack_data.check_ray_mask = BitBoard(0);
        } else {
            let checker_sq = checkers.lsb().unwrap() as usize;
            let ray = MOVE_TABLES.get_ray_between(king_sq, checker_sq);
            attack_data.check_ray_mask = ray | checkers;
        }
    }

    let xray_occupancy = occupied ^ *friendly_pieces;

    let xray_ortho = MOVE_TABLES.get_rook_attacks_bb(king_sq, xray_occupancy);
    let xray_diag = MOVE_TABLES.get_bishop_attacks_bb(king_sq, xray_occupancy);

    let pinners = (xray_ortho & opp_rooks_queens) | (xray_diag & opp_bishops_queens);

    for pinner_sq in pinners.iter_bits() {
        let ray = MOVE_TABLES.get_ray_between(king_sq, pinner_sq);

        let pinned_on_ray = ray & *friendly_pieces;

        if pinned_on_ray.pop_count() == 1 {
            attack_data.pin_ray_mask |= ray | BitBoard(1 << pinner_sq);
        }
    }

    attack_data
}

/// Returns BitBoard with all pieces attacking the king
pub fn calculate_opp_attack_map(board: &Board, side: Side) -> BitBoard {
    let opponent = side.flip();
    let occupied = board.positions.get_occupied_bb();

    let king_sq = board
        .positions
        .get_piece_bb(side, Piece::King)
        .lsb()
        .unwrap_or(0) as usize;

    let occupancy_no_king = occupied ^ BitBoard(1 << king_sq);

    let mut attack_map = BitBoard(0);

    let opp_rook_queens = board.positions.get_ortho_sliders_bb(opponent);
    for sq in opp_rook_queens.iter_bits() {
        attack_map |= MOVE_TABLES.get_rook_attacks_bb(sq, occupancy_no_king);
    }

    let opp_bishop_queens = board.positions.get_diag_sliders_bb(opponent);
    for sq in opp_bishop_queens.iter_bits() {
        attack_map |= MOVE_TABLES.get_bishop_attacks_bb(sq, occupancy_no_king);
    }
    let opp_knights = board.positions.get_piece_bb(opponent, Piece::Knight);
    for sq in opp_knights.iter_bits() {
        attack_map |= MOVE_TABLES.knight_moves[sq];
    }
    let opp_pawns = board.positions.get_piece_bb(opponent, Piece::Pawn);
    for sq in opp_pawns.iter_bits() {
        attack_map |= MOVE_TABLES.get_pawn_attacks(sq, opponent)
    }
    if let Some(opp_king_sq) = board.positions.get_piece_bb(opponent, Piece::King).lsb() {
        attack_map |= MOVE_TABLES.king_moves[opp_king_sq as usize];
    }
    attack_map
}
