use crate::{Board, Piece, consts::NUM_PIECES, moves::move_info::Move};

const VICTIM_SCORES: [i32; NUM_PIECES] = [
    100,    // Pawn
    320,    // Knight
    330,    // Bishop
    500,    // Rook
    900,    // Queen
    20_000, // King
];

// Using large offsets to create distinct "buckets" for move types.
// This ensures that any capture is scored higher than any killer move, etc.
const MVV_LVA_OFFSET: i32 = 2_000_000;
const KILLER_MOVE_SCORE: i32 = 1_000_000;
// TODO: Add more scores here

pub fn score_move(board: &Board, mv: Move, killers: &[Option<Move>; 2]) -> i32 {
    if mv.is_capture() {
        let attacker = board.get_piece_at(mv.from_sq()).unwrap_or_default();
        let victim = if mv.is_enpassant() {
            Piece::Pawn
        } else {
            board.get_piece_at(mv.to_sq()).unwrap_or_default()
        };
        // Most Valuable Victim - Least Valuable Attacker
        MVV_LVA_OFFSET + VICTIM_SCORES[victim.index()] - VICTIM_SCORES[attacker.index()]
    } else if killers.contains(&Some(mv)) {
        KILLER_MOVE_SCORE
    } else {
        // TODO: History heuristic goes here
        0
    }
}

// Sorts a slice of moves in-place from best to worst based on their score
pub fn sort_moves(board: &Board, moves: &mut [Move], killers: &[Option<Move>; 2]) {
    // Score is negated here because sort is ascending, but we want descending
    // PERF: Maybe unstable_sort_by_key is faster?
    moves.sort_by_key(|&m| -score_move(board, m, killers));
}
