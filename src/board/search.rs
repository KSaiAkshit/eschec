use super::*;
use std::cmp::max;
use std::i32;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct SearchResult {
    pub best_move: Option<(Square, Square)>,
    pub score: i32,
    pub depth: u8,
    pub nodes_searched: u64,
    pub time_taken: Duration,
}

pub struct Search {
    max_depth: u8,
    nodes_searched: u64,
    start_time: Instant,
}

impl Search {
    pub fn new(max_depth: u8) -> Self {
        Self {
            max_depth,
            nodes_searched: 0,
            start_time: Instant::now(),
        }
    }

    #[allow(unused_mut)]
    #[instrument(skip_all)]
    pub fn find_best_move(&mut self, board: &Board, evaluator: &dyn Evaluator) -> SearchResult {
        self.nodes_searched = 0;
        self.start_time = Instant::now();

        let mut best_move = None;
        let mut alpha = i32::MIN + 1;
        let mut beta = i32::MAX;

        let legal_moves = match board.generate_legal_moves() {
            Ok(moves) => moves,
            Err(_) => {
                return SearchResult {
                    best_move: None,
                    score: 0,
                    depth: self.max_depth,
                    nodes_searched: 0,
                    time_taken: Duration::from_secs(0),
                }
            }
        };
        if legal_moves.is_empty() {
            return SearchResult {
                best_move: None,
                score: if board.is_in_check(board.stm) {
                    -20000
                } else {
                    0
                }, // Checkmate or stalemate
                depth: self.max_depth,
                nodes_searched: 0,
                time_taken: Duration::from_secs(0),
            };
        }

        let mut best_score = i32::MIN + 1;

        for (from, to) in legal_moves {
            let mut board_copy = *board;
            if board_copy.make_move(from, to).is_err() {
                continue;
            }

            let score = -self.alpha_beta(&board_copy, self.max_depth - 1, -beta, -alpha, evaluator);

            if score > best_score {
                best_score = score;
                best_move = Some((from, to));
            }

            alpha = max(alpha, best_score);
            if alpha >= beta {
                break;
            }
        }
        let time_taken = self.start_time.elapsed();

        SearchResult {
            best_move,
            score: best_score,
            depth: self.max_depth,
            nodes_searched: self.nodes_searched,
            time_taken,
        }
    }

    #[allow(unused_mut)]
    #[instrument(skip_all)]
    fn alpha_beta(
        &mut self,
        board: &Board,
        depth: u8,
        mut alpha: i32,
        mut beta: i32,
        evaluator: &dyn Evaluator,
    ) -> i32 {
        self.nodes_searched += 1;
        info!(self.nodes_searched);

        if depth == 0 {
            return evaluator.evaluate(board);
        }

        let legal_moves = match board.generate_legal_moves() {
            Ok(moves) => moves,
            Err(_) => return 0,
        };

        if legal_moves.is_empty() {
            return if board.is_in_check(board.stm) {
                -20_000 + (self.max_depth - depth) as i32 // prefer faster checkmate
            } else {
                0 // stalemate
            };
        }

        let mut best_score = i32::MIN + 1;

        for (from, to) in legal_moves {
            let mut board_copy = *board;
            if board_copy.make_move(from, to).is_err() {
                continue;
            }

            let score = -self.alpha_beta(&board_copy, depth - 1, -beta, -alpha, evaluator);

            best_score = max(best_score, score);
            alpha = max(alpha, best_score);

            if alpha >= beta {
                break;
            }
        }
        best_score
    }
}
