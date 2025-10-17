use std::mem::MaybeUninit;

use crate::prelude::*;

// Using large offsets to create distinct "buckets" for move types.
// This ensures that any capture is scored higher than any killer move, etc.
const TT_MOVE_SCORE: i32 = 3_000_000;
const MVV_LVA_OFFSET: i32 = 2_000_000;
const KILLER_MOVE_SCORE: i32 = 1_000_000;
const PROMOTION_SCORE: i32 = 1_000_000;
const BAD_CAPTURE_SCORE: i32 = 900_000;

pub trait MoveScoringPolicy {
    fn score(
        board: &Board,
        mv: Move,
        killers: &[Option<Move>; 2],
        tt_move: Option<Move>,
        history: &[[i32; NUM_SQUARES]; NUM_SQUARES],
    ) -> i32;
}

pub struct MainSearchPolicy;

impl MoveScoringPolicy for MainSearchPolicy {
    fn score(
        board: &Board,
        mv: Move,
        killers: &[Option<Move>; 2],
        tt_move: Option<Move>,
        history: &[[i32; NUM_SQUARES]; NUM_SQUARES],
    ) -> i32 {
        if tt_move.is_some_and(|tt_mv| tt_mv == mv) {
            return TT_MOVE_SCORE;
        }
        if mv.is_capture() {
            let see_score = board.static_exchange_evaluation(mv);
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
}

pub struct QSearchPolicy;

impl MoveScoringPolicy for QSearchPolicy {
    fn score(
        board: &Board,
        mv: Move,
        _killers: &[Option<Move>; 2],
        _tt_move: Option<Move>,
        _history: &[[i32; NUM_SQUARES]; NUM_SQUARES],
    ) -> i32 {
        if mv.is_capture() {
            MVV_LVA_OFFSET + board.static_exchange_evaluation(mv)
        } else if mv.is_promotion() {
            if mv.promoted_piece() == Some(Piece::Queen) {
                PROMOTION_SCORE
            } else {
                PROMOTION_SCORE / 2
            }
        } else {
            0
        }
    }
}

/// Sorts a slice of moves in-place from best to worst based on their score
pub fn sort_moves<P: MoveScoringPolicy>(
    board: &Board,
    moves: &mut [Move],
    killers: &[Option<Move>; 2],
    tt_move: Option<Move>,
    history: &[[i32; 64]; 64],
    seed: u64,
) {
    let num_moves = moves.len();
    let mut scored_moves: [MaybeUninit<(i32, Move)>; MAX_MOVES] =
        unsafe { MaybeUninit::uninit().assume_init() };
    let mut prng = Prng::init(seed);

    for i in 0..num_moves {
        let base_score = P::score(board, moves[i], killers, tt_move, history);
        let final_score = base_score.saturating_add((prng.rand() % 10) as i32);
        scored_moves[i].write((-final_score, moves[i])); // Negate for descending sort
    }

    let scored_slice = unsafe {
        let ptr = scored_moves.as_mut_ptr() as *mut (i32, Move);
        std::slice::from_raw_parts_mut(ptr, num_moves)
    };

    scored_slice.sort_unstable_by_key(|(score, _)| *score);

    for i in 0..num_moves {
        moves[i] = scored_slice[i].1;
    }
}
