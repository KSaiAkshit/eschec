use crate::consts::{MATE_THRESHOLD, MAX_PLY};
use crate::search::move_ordering::sort_moves;
use crate::search::tt::{ScoreTypes, TranspositionEntry, TranspositionTable};
use crate::{evaluation::Evaluator, moves::move_info::Move};
use tracing::*;

pub mod move_ordering;
pub mod tt;

use super::*;
use std::cmp::{max, min};
use std::i32;
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
    tt: TranspositionTable,
    killer_moves: [[Option<Move>; 2]; MAX_PLY],
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
            tt: TranspositionTable::new(16),
            killer_moves: [[None; 2]; MAX_PLY],
        }
    }

    pub fn with_time_control(max_depth: u8, max_time_ms: u64) -> Self {
        let tt = TranspositionTable::new(16);
        println!("Size of table: {}", std::mem::size_of_val(&tt));
        Self {
            max_depth,
            nodes_searched: 0,
            start_time: Instant::now(),
            max_time: Some(Duration::from_millis(max_time_ms)),
            nodes_limit: None,
            pruned_nodes: 0,
            search_running: None,
            tt,
            killer_moves: [[None; 2]; MAX_PLY],
        }
    }

    pub fn change_depth(&mut self, new_max_depth: u8) -> miette::Result<()> {
        miette::ensure!(
            (new_max_depth as usize) < MAX_PLY,
            "New depth ({new_max_depth}) cannot be greater than {MAX_PLY}"
        );
        self.max_depth = new_max_depth;
        Ok(())
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
        self.killer_moves = [[None; 2]; MAX_PLY];

        let mut legal_moves = board.generate_legal_moves(false);
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

            sort_moves(board, &mut legal_moves, &[None, None], None);

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
                let score = -self.alpha_beta(&board_copy, depth - 1, 0, -beta, -alpha, evaluator);

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
                debug!(msg);
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
        ply: usize,
        mut alpha: i32,
        mut beta: i32,
        evaluator: &dyn Evaluator,
    ) -> i32 {
        if self.should_stop() {
            return 0; // Neutral score because search was stopped
        }

        let original_alpha = alpha;

        let current_hash = board.hash;
        let mut tt_move = Move::default();

        if let Some(entry) = self.tt.probe(current_hash)
            && entry.hash == current_hash
        {
            tt_move = entry.best_move;
            if entry.depth >= depth {
                let score = adjust_score_for_ply(entry.score, ply);

                match entry.score_type {
                    tt::ScoreTypes::Exact => {
                        // Exact score can be returned as is
                        return score;
                    }
                    tt::ScoreTypes::LowerBound => {
                        // The true score is at least 'prev_score'
                        // if this is enough to beat beta, we can prune
                        alpha = max(alpha, score);
                        if alpha >= beta {
                            return beta;
                        }
                    }
                    tt::ScoreTypes::UpperBound => {
                        // The true score is at most 'prev_score'
                        // if this is enough to beat alpha, we can prune
                        beta = min(beta, score);
                        if alpha >= beta {
                            return alpha;
                        }
                    }
                }
            }
        }

        if depth == 0 {
            return self.quiescence_search(board, alpha, beta, evaluator);
        }

        let mut legal_moves = board.generate_legal_moves(false);

        if legal_moves.is_empty() {
            self.nodes_searched += 1; // Terminal nodes_search
            return if board.is_in_check(board.stm) {
                -20_000 + ply as i32 // Checkmate
            } else {
                0 // Stalemate
            };
        }

        if ply < MAX_PLY {
            sort_moves(
                board,
                &mut legal_moves,
                &self.killer_moves[ply],
                Some(tt_move),
            );
        }

        let mut best_move_this_node = legal_moves.first().copied().unwrap();
        let mut best_score = i32::MIN + 1;

        for mv in legal_moves {
            let mut board_copy = *board;
            board_copy
                .make_move(mv)
                .expect("Should be safe since we use legal moves");

            self.nodes_searched += 1;

            let score = -self.alpha_beta(&board_copy, depth - 1, ply + 1, -beta, -alpha, evaluator);

            if self.should_stop() {
                return 0;
            }

            if score > best_score {
                best_score = score;
                best_move_this_node = mv;
            }

            alpha = max(alpha, score);

            if alpha >= beta {
                self.pruned_nodes += 1;

                // This is beta cutoff (Fail-High)
                // if move was quiet, it is a good candidate for killer move
                if !mv.is_capture() && ply < MAX_PLY {
                    // Backup the existing one
                    self.killer_moves[ply][1] = self.killer_moves[ply][0];
                    self.killer_moves[ply][0] = Some(mv);
                }

                let entry_to_score = TranspositionEntry {
                    hash: current_hash,
                    depth,
                    score: adjust_score_from_ply(beta, ply),
                    score_type: ScoreTypes::LowerBound,
                    best_move: best_move_this_node,
                };
                self.tt.store(entry_to_score);
                return beta;
            }
        }

        let score_type = if best_score <= original_alpha {
            // We failed to raise alpha. This is a "fail-low".
            // The score is an upper bound on the node's true value.
            ScoreTypes::UpperBound // Fail-low
        } else {
            // Successfully raised alpha but did not fail high.
            // This means the exact best score was found in the (alpha, beta) window.
            ScoreTypes::Exact // Exact score
        };

        let entry_to_store = TranspositionEntry {
            hash: current_hash,
            depth,
            score: adjust_score_from_ply(best_score, ply),
            score_type,
            best_move: best_move_this_node,
        };
        self.tt.store(entry_to_store);

        alpha
    }

    fn quiescence_search(
        &mut self,
        board: &Board,
        mut alpha: i32,
        beta: i32,
        evaluator: &dyn Evaluator,
    ) -> i32 {
        self.nodes_searched += 1;

        if self.should_stop() {
            return 0;
        }

        let stand_pat_score = evaluator.evaluate(board);

        if stand_pat_score > beta {
            self.pruned_nodes += 1;
            return beta; // Fail high
        }
        alpha = max(alpha, stand_pat_score);

        let mut legal_moves = board.generate_legal_moves(true);

        sort_moves(board, &mut legal_moves, &[None, None], None);

        for mv in legal_moves {
            let mut board_copy = *board;
            if board_copy.make_move(mv).is_err() {
                continue;
            }

            let score = -self.quiescence_search(&board_copy, -beta, -alpha, evaluator);

            if score >= beta {
                self.pruned_nodes += 1;
                return beta; // Fail high
            }
            alpha = max(alpha, score)
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

fn adjust_score_for_ply(score: i32, ply: usize) -> i32 {
    if score.abs() > MATE_THRESHOLD {
        if score > 0 {
            score - ply as i32
        } else {
            score + ply as i32
        }
    } else {
        score
    }
}

fn adjust_score_from_ply(score: i32, ply: usize) -> i32 {
    if score.abs() > MATE_THRESHOLD {
        if score > 0 {
            score + ply as i32
        } else {
            score - ply as i32
        }
    } else {
        score
    }
}
