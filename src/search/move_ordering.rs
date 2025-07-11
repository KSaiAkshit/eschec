use crate::{Board, Piece, consts::NUM_PIECES, moves::move_info::Move};

const VICTIM_SCORES: [i32; NUM_PIECES] = [100, 320, 330, 500, 900, 20_000];

const MVV_LVA_OFFSET: i32 = 2_000_000;
const KILLER_MOVE_SCORE: i32 = 1_000_000;

pub fn score_move(board: &Board, mov: Move, killers: &[Option<Move>; 2]) -> i32 {
    if mov.is_capture() {
        let attacker = board.get_piece_at(mov.from_sq()).unwrap_or_default();
        let victim = if mov.is_enpassant() {
            Piece::Pawn
        } else {
            board.get_piece_at(mov.to_sq()).unwrap_or_default()
        };
        // Most Valuable Victim - Least Valuable Attacker
        MVV_LVA_OFFSET + VICTIM_SCORES[victim.index()] - attacker.value() as i32
    } else if killers.contains(&Some(mov)) {
        KILLER_MOVE_SCORE
    } else {
        // NOTE: Is this correct? Maybe something that is better
        0
    }
}

pub fn sort_moves(board: &Board, moves: &mut [Move], killers: &[Option<Move>; 2]) {
    moves.sort_unstable_by_key(|&m| -score_move(board, m, killers));
}
