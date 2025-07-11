use crate::{evaluation::Evaluator, moves::move_info::Move};
use tracing::*;

pub mod zobrist;
pub mod tt;
pub mod move_ordering;

use super::*;
use std::cmp::max;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct SearchResult {
    pub best_move: Option<Move>,
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
    search_running: Option<Arc<AtomicBool>>,
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
            search_running: None,
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
            search_running: None,
        }
    }

    pub fn change_depth(&mut self, new_max_depth: u8) {
        self.max_depth = new_max_depth;
    }

    // #[instrument(skip_all)]
    pub fn find_best_move(
        &mut self,
        board: &Board,
        evaluator: &dyn Evaluator,
        search_running: Option<Arc<AtomicBool>>,
    ) -> SearchResult {
        let span = trace_span!("search_root");
        let _guard = span.enter();

        self.nodes_searched = 0;
        self.start_time = Instant::now();
        self.search_running = search_running;

        let legal_moves = board.generate_legal_moves();
        if legal_moves.is_empty() {
            debug!("No legal moves");
            let score = if board.is_in_check(board.stm) {
                -20000 // checkmate
            } else {
                0 // stalemate
            };
            return SearchResult {
                best_move: None,
                score,
                depth: self.max_depth,
                nodes_searched: 0,
                time_taken: Duration::from_secs(0),
            };
        }

        // Initialized to first move as fallback
        let mut best_move = legal_moves.first().copied();
        let mut best_score = i32::MIN + 1;
        let mut completed_depth = u8::default();

        // Iterative deepening
        for depth in 1..=self.max_depth {
            if self.should_stop() {
                break;
            }

            let mut alpha = i32::MIN + 1;
            let beta = i32::MAX;

            let mut local_best_move: Option<Move> = legal_moves.first().copied();
            let mut local_best_score = i32::MIN + 1;

            for &m in &legal_moves {
                if self.should_stop() {
                    break;
                }

                let mut board_copy = *board;
                if board_copy.make_move(m).is_err() {
                    continue;
                }

                debug!("Evaluating move: {}", m);
                let score = -self.alpha_beta(&board_copy, depth - 1, -beta, -alpha, evaluator);

                if self.should_stop() {
                    break;
                }

                if score > local_best_score {
                    local_best_score = score;
                    local_best_move = Some(m);
                }

                alpha = max(alpha, local_best_score);
            }

            if !self.should_stop() {
                completed_depth = depth;
                best_move = local_best_move;
                best_score = local_best_score;

                let best_move_uci = best_move.unwrap().uci();
                let msg = format!(
                    "info depth {} score cp {} nodes {} nps {} pv {}",
                    depth,
                    best_score,
                    self.nodes_searched,
                    self.nodes_searched * 1000 / (self.start_time.elapsed().as_millis() + 1) as u64,
                    best_move_uci
                );
                info!(msg);
                println!("{msg}");
            }
        }

        SearchResult {
            best_move,
            score: best_score,
            depth: completed_depth,
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
        if self.should_stop() {
            return alpha;
        }
        self.nodes_searched += 1;

        if depth == 0 {
            let score = evaluator.evaluate(board);
            trace!("Returning static eval: {}", score);
            return score;
        }

        let legal_moves = board.generate_legal_moves();

        if legal_moves.is_empty() {
            return if board.is_in_check(board.stm) {
                -20_000 + (self.max_depth - depth) as i32 // prefer faster checkmate
            } else {
                0 // stalemate
            };
        }

        for m in legal_moves {
            let mut board_copy = *board;
            if board_copy.make_move(m).is_err() {
                continue;
            }

            let score = -self.alpha_beta(&board_copy, depth - 1, -beta, -alpha, evaluator);

            if alpha >= beta {
                self.pruned_nodes += 1;
                return beta;
            }
            alpha = max(alpha, score);
        }
        alpha
    }

    fn should_stop(&self) -> bool {
        if let Some(flag) = &self.search_running
            && !flag.load(Ordering::Relaxed)
        {
            return true;
        }

        if let Some(max_time) = self.max_time
            && self.start_time.elapsed() >= max_time
        {
            return true;
        }
        self.nodes_limit.is_some_and(|l| self.nodes_searched >= l)
    }
}
