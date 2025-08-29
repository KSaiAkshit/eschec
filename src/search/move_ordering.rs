use crate::prelude::*;

// Using large offsets to create distinct "buckets" for move types.
// This ensures that any capture is scored higher than any killer move, etc.
const TT_MOVE_SCORE: i32 = 3_000_000;
const MVV_LVA_OFFSET: i32 = 2_000_000;
const KILLER_MOVE_SCORE: i32 = 1_000_000;
const BAD_CAPTURE_SCORE: i32 = 900_000;
// TODO: Add more scores here

pub fn score_move(
    board: &Board,
    mv: Move,
    killers: &[Option<Move>; 2],
    tt_move: Option<Move>,
    history: &[[i32; 64]; 64],
) -> i32 {
    if tt_move.is_some_and(|tt_mv| tt_mv == mv) {
        return TT_MOVE_SCORE;
    }
    if mv.is_capture() {
        let see_score = board.see(mv);
        if see_score > 0 {
            // Most Valuable Victim - Least Valuable Attacker
            MVV_LVA_OFFSET + see_score
        } else {
            BAD_CAPTURE_SCORE + see_score
        }
    } else if killers.contains(&Some(mv)) {
        KILLER_MOVE_SCORE
    } else {
        history[mv.from_idx() as usize][mv.to_idx() as usize]
    }
}

// Sorts a slice of moves in-place from best to worst based on their score
pub fn sort_moves(
    board: &Board,
    moves: &mut [Move],
    killers: &[Option<Move>; 2],
    tt_move: Option<Move>,
    history: &[[i32; 64]; 64],
    seed: u64,
) {
    let mut prng = Prng::init(seed);

    let mut scored_moves: Vec<(i32, Move)> = moves
        .iter()
        .map(|&m| {
            let base_score = score_move(board, m, killers, tt_move, history);
            let final_score = base_score + (prng.rand() % 10) as i32;
            (-final_score, m) // Negate for descending sort
        })
        .collect();

    scored_moves.sort_unstable_by_key(|(score, _)| *score);
    for (i, &(_, mv)) in scored_moves.iter().enumerate() {
        moves[i] = mv;
    }
}
