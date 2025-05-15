use crate::evaluation::Evaluator;
use tracing::*;

use super::*;
use std::cmp::max;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct SearchResult {
    pub best_move: Option<(Square, Square)>,
    pub score: i32,
    pub depth: u8,
    pub nodes_searched: u64,
    pub time_taken: Duration,
}

#[derive(Debug)]
pub struct Search {
    max_depth: u8,
    nodes_searched: u64,
    start_time: Instant,
    max_time: Option<Duration>,
    nodes_limit: Option<u64>,
    pruned_nodes: u64,
}

impl Search {
    pub fn new(max_depth: u8) -> Self {
        Self {
            max_depth,
            nodes_searched: 0,
            start_time: Instant::now(),
            max_time: None,
            nodes_limit: None,
            pruned_nodes: 0,
        }
    }

    pub fn with_time_control(max_depth: u8, max_time_ms: u64) -> Self {
        Self {
            max_depth,
            nodes_searched: 0,
            start_time: Instant::now(),
            max_time: Some(Duration::from_millis(max_time_ms)),
            nodes_limit: None,
            pruned_nodes: 0,
        }
    }

    pub fn change_depth(&mut self, new_max_depth: u8) {
        self.max_depth = new_max_depth;
    }

    // #[instrument(skip_all)]
    pub fn find_best_move(&mut self, board: &Board, evaluator: &dyn Evaluator) -> SearchResult {
        let span = info_span!("search_root");
        let _guard = span.enter();
        self.nodes_searched = 0;
        self.start_time = Instant::now();

        let legal_moves = match board.generate_legal_moves() {
            Ok(moves) => moves,
            Err(e) => {
                error!("Error generating legal moves");
                eprintln!("Error: {:?}", e);
                return SearchResult {
                    best_move: None,
                    score: 0,
                    depth: self.max_depth,
                    nodes_searched: 0,
                    time_taken: Duration::from_secs(0),
                };
            }
        };
        if legal_moves.is_empty() {
            debug!("No legal moves");
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

        let mut best_move = None;
        let mut best_score = i32::MIN + 1;
        let mut completed_depth = 0;

        for depth in 1..=self.max_depth {
            let mut local_best_move: Option<(Square, Square)> = None;
            let mut local_best_score = i32::MIN + 1;
            let mut alpha = i32::MIN + 1;
            let beta = i32::MAX;
            for (from, to) in &legal_moves {
                if self.is_time_up() || self.node_limit_reached() {
                    return SearchResult {
                        best_move,
                        score: best_score,
                        depth: completed_depth,
                        nodes_searched: self.nodes_searched,
                        time_taken: self.start_time.elapsed(),
                    };
                }
                let mut board_copy = *board;
                if board_copy.try_move(*from, *to).is_err() {
                    continue;
                }

                info!("Evaluating from: {}, to: {}", from, to);
                let score = -self.alpha_beta(&board_copy, depth - 1, -beta, -alpha, evaluator);

                if score > local_best_score {
                    local_best_score = score;
                    local_best_move = Some((*from, *to));
                }

                alpha = max(alpha, local_best_score);
                if alpha >= beta {
                    warn!("alpha {alpha} > beta {beta}");
                    break;
                }
            }

            if local_best_move.is_some() {
                best_move = local_best_move;
                best_score = local_best_score;
                completed_depth = depth;
            }
        }

        SearchResult {
            best_move,
            score: best_score,
            depth: self.max_depth,
            nodes_searched: self.nodes_searched,
            time_taken: self.start_time.elapsed(),
        }
    }

    #[instrument(skip_all)]
    fn alpha_beta(
        &mut self,
        board: &Board,
        depth: u8,
        mut alpha: i32,
        beta: i32,
        evaluator: &dyn Evaluator,
    ) -> i32 {
        if self.is_time_up() || self.node_limit_reached() {
            return alpha;
        }
        self.nodes_searched += 1;
        info!(self.nodes_searched);

        if depth == 0 {
            let score = evaluator.evaluate(board);
            trace!("Returning static eval: {}", score);
            return score;
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
            if board_copy.try_move(from, to).is_err() {
                continue;
            }

            let score = -self.alpha_beta(&board_copy, depth - 1, -beta, -alpha, evaluator);

            best_score = max(best_score, score);
            alpha = max(alpha, best_score);

            if alpha >= beta {
                self.pruned_nodes += 1;
                break;
            }
        }
        best_score
    }

    fn is_time_up(&self) -> bool {
        if let Some(max_time) = self.max_time {
            self.start_time.elapsed() >= max_time
        } else {
            false
        }
    }

    fn node_limit_reached(&self) -> bool {
        // NOTE: Is this required?
        self.nodes_limit.is_some_and(|l| self.nodes_searched >= l)
    }
}
