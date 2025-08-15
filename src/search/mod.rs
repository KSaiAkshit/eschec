use crate::search::move_ordering::sort_moves;
use crate::search::tt::{ScoreTypes, TranspositionEntry, TranspositionTable};
use tracing::*;

pub mod move_ordering;
pub mod tt;

use crate::prelude::*;
use std::cmp::{max, min};
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
    pub pruned_nodes: u64,
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
    history: [[i32; 64]; 64],
    hash_history: Vec<u64>, // TODO: Should this be a stack thing instead // of a Heap allocated Vec
    enable_nmp: bool,
    emit_info: bool,
    in_progress: bool,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            max_depth: Default::default(),
            nodes_searched: Default::default(),
            start_time: Instant::now(),
            max_time: Default::default(),
            nodes_limit: Default::default(),
            pruned_nodes: Default::default(),
            search_running: Default::default(),
            tt: TranspositionTable::new(16),
            killer_moves: [[None; 2]; 64],
            history: [[0; 64]; 64],
            hash_history: Default::default(),
            enable_nmp: true,
            emit_info: true,
            in_progress: false,
        }
    }
}

impl Search {
    pub fn new(max_depth: u8) -> Self {
        Self {
            max_depth,
            ..Default::default()
        }
    }

    pub fn with_time_control(max_depth: u8, max_time_ms: u64) -> Self {
        Self {
            max_depth,
            max_time: Some(Duration::from_millis(max_time_ms)),
            ..Default::default()
        }
    }

    pub fn init(self, search_running: Option<Arc<AtomicBool>>) -> Self {
        Self {
            search_running,
            ..self
        }
    }

    pub fn set_depth(&mut self, new_max_depth: u8) -> miette::Result<()> {
        miette::ensure!(
            (new_max_depth as usize) < MAX_PLY,
            "New depth ({new_max_depth}) cannot be greater than {MAX_PLY}"
        );
        self.max_depth = new_max_depth;
        self.max_time = None;
        warn!(
            "New depth set to {:?}, time limit set to: {:?}",
            self.max_depth, self.max_time
        );
        Ok(())
    }

    pub fn set_time(&mut self, max_time_ms: u64) -> miette::Result<()> {
        miette::ensure!(
            !self.in_progress,
            "Search already running, cannot change time limit!"
        );
        self.max_time = Some(Duration::from_millis(max_time_ms));
        self.max_depth = (MAX_PLY - 1) as u8;
        warn!(
            "New time limit set to {:?}, max depth set to {}",
            self.max_time, self.max_depth
        );
        Ok(())
    }

    pub fn change_hash_size(&mut self, new_size_mb: usize) -> miette::Result<()> {
        self.tt.change_size(new_size_mb)?;
        Ok(())
    }

    pub fn toggle_nmp(&mut self) -> bool {
        if !self.in_progress {
            self.enable_nmp = !self.enable_nmp;
            true
        } else {
            false
        }
    }

    pub fn set_emit_info(&mut self, enabled: bool) {
        debug!("setting emit_info to: {enabled}");
        self.emit_info = enabled;
    }

    #[instrument(skip_all)]
    pub fn find_best_move(&mut self, board: &Board, evaluator: &dyn Evaluator) -> SearchResult {
        self.start();

        let span = trace_span!("search_root");
        let _guard = span.enter();

        info!("Finding best move");

        self.prepare_for_search();
        self.start_time = Instant::now();
        self.hash_history.push(board.hash);

        let mut legal_moves = MoveBuffer::new();
        board.generate_legal_moves(&mut legal_moves, false);
        if legal_moves.is_empty() {
            debug!("No legal moves");
            let score = if board.is_in_check(board.stm) {
                -20000 // checkmate
            } else {
                0 // stalemate
            };
            self.hash_history.pop();
            self.finish();
            return SearchResult {
                best_move: None,
                score,
                depth: self.max_depth,
                nodes_searched: 0,
                time_taken: Duration::from_secs(0),
                pruned_nodes: self.pruned_nodes,
            };
        }

        // Initialized to first move as fallback
        let mut best_move = legal_moves.first();
        let mut best_score = i32::MIN + 1;
        let mut completed_depth = u8::default();

        // Iterative deepening
        for depth in 1..=self.max_depth {
            if self.should_stop() {
                break;
            }

            let seed = board.hash.wrapping_add(depth as u64);

            let mut alpha = i32::MIN + 1;
            let beta = i32::MAX;

            sort_moves(
                board,
                legal_moves.as_mut_slice(),
                &[None, None],
                None,
                &self.history,
                seed,
            );

            let mut local_best_move: Option<Move> = legal_moves.first();
            let mut local_best_score = i32::MIN + 1;

            for &m in &legal_moves {
                if self.should_stop() {
                    break;
                }

                let mut board_copy = *board;
                if board_copy.make_move(m).is_err() {
                    // Ply 1
                    continue;
                }
                self.hash_history.push(board_copy.hash);

                trace!("Evaluating move: {}, α: {alpha}, β: {beta}", m.uci());
                let score = -self.alpha_beta(&board_copy, depth - 1, 1, -beta, -alpha, evaluator);

                self.hash_history.pop();

                if self.should_stop() {
                    break;
                }

                if score > local_best_score {
                    local_best_score = score;
                    local_best_move = Some(m);
                }

                alpha = max(alpha, local_best_score);
            }

            if !self.should_stop() && self.emit_info {
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
                debug!("{msg}");
                println!("{msg}");
            }
        }

        self.hash_history.pop();
        self.decay_history();
        self.finish();
        SearchResult {
            best_move,
            score: best_score,
            depth: completed_depth,
            nodes_searched: self.nodes_searched,
            time_taken: self.start_time.elapsed(),
            pruned_nodes: self.pruned_nodes,
        }
    }

    /// Reset Search state without clobbering TranspositionTable
    fn prepare_for_search(&mut self) {
        self.nodes_searched = 0;
        self.pruned_nodes = 0;
        self.killer_moves = [[None; 2]; MAX_PLY];
        self.hash_history.clear();
        self.decay_history();
    }

    // #[instrument(skip_all)]
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

        if depth == 0 {
            return self.quiescence_search(board, ply, alpha, beta, evaluator);
        }

        if alpha >= beta {
            debug_assert!(
                false,
                "Invalid alpha-beta window: alpha: {alpha}, beta: {beta}"
            );
            return alpha;
        }

        let original_alpha = alpha;
        let is_in_check = board.is_in_check(board.stm);

        let current_hash = board.hash;
        let repetition_count = self
            .hash_history
            .iter()
            .filter(|&&hash| hash == current_hash)
            .count();

        if repetition_count >= 2 || board.halfmove_clock >= 100 {
            return 0;
        }
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

        if alpha >= beta {
            debug_assert!(
                false,
                "Invalid initial alpha-beta window: alpha: {alpha}, beta: {beta}"
            );
            return alpha;
        }

        let window = beta.checked_sub(alpha);

        let null_reduction = if depth >= 6 { 4 } else { 2 };
        let null_depth = depth.saturating_sub(null_reduction);
        if self.is_nmp_allowed(board, depth, ply, window) && !is_in_check {
            debug!(
                "NMP => depth: {depth}, null_reduction: {null_reduction}, null_depth: {null_depth}"
            );
            let mut null_board = *board;
            null_board.make_null_move();

            let null_score = -self.alpha_beta(
                &null_board,
                null_depth,
                ply + 1,
                -beta,
                -beta + 1,
                evaluator,
            );

            if null_score >= beta {
                return beta;
            }
        }

        let mut legal_moves = MoveBuffer::new();
        board.generate_legal_moves(&mut legal_moves, false);

        if legal_moves.is_empty() {
            self.nodes_searched += 1; // Terminal nodes_search
            return if board.is_in_check(board.stm) {
                -20_000 + ply as i32 // Checkmate
            } else {
                0 // Stalemate
            };
        }

        if ply < MAX_PLY {
            let seed = board.hash.wrapping_add(ply as u64);
            sort_moves(
                board,
                legal_moves.as_mut_slice(),
                &self.killer_moves[ply],
                Some(tt_move),
                &self.history,
                seed,
            );
        }

        let mut best_move_this_node = legal_moves.first().unwrap();
        let mut best_score = i32::MIN + 1;
        let num_moves = legal_moves.len();

        for mv in legal_moves {
            let mut board_copy = *board;
            board_copy
                .make_move(mv)
                .expect("Should be safe since we use legal moves");

            self.nodes_searched += 1;

            self.hash_history.push(board_copy.hash);

            let mut score: i32;
            // Principal Variation search.
            // Use ZWS (zero window search) for moves other than the first move (PV)
            // Reduces nodes searched massively
            if mv == best_move_this_node {
                score = -self.alpha_beta(&board_copy, depth - 1, ply + 1, -beta, -alpha, evaluator);
            } else {
                score = -self.alpha_beta(
                    &board_copy,
                    depth - 1,
                    ply + 1,
                    -alpha - 1,
                    -alpha,
                    evaluator,
                );
                if score > alpha && score < beta {
                    score =
                        -self.alpha_beta(&board_copy, depth - 1, ply + 1, -beta, -alpha, evaluator);
                }
            }

            self.hash_history.pop();

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

                    let from = mv.from_idx() as usize;
                    let to = mv.to_idx() as usize;
                    self.history[from][to] += depth as i32 * depth as i32;
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

        if best_score == i32::MIN + 1 {
            error!(
                "Mad cooked: alpha: {alpha}, beta: {beta}, best_score: {best_score}, legal_moves searched: {}",
                num_moves
            );
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

    fn is_nmp_allowed(&self, board: &Board, depth: u8, ply: usize, window: Option<i32>) -> bool {
        let null_reduction = if depth >= 6 { 4 } else { 2 };
        let null_depth = depth.saturating_sub(null_reduction);
        self.enable_nmp
            && depth >= 5
            && null_depth >= 2
            && window.is_some_and(|win| win > 1)
            && has_non_pawn_material(board)
            && ply > 0
    }

    fn quiescence_search(
        &mut self,
        board: &Board,
        ply: usize,
        mut alpha: i32,
        beta: i32,
        evaluator: &dyn Evaluator,
    ) -> i32 {
        self.nodes_searched += 1;

        if ply >= 3 {
            return evaluator.evaluate(board);
        }

        if self.should_stop() {
            return 0;
        }

        let current_hash = board.hash;
        let repetition_count = self
            .hash_history
            .iter()
            .filter(|&&hash| hash == current_hash)
            .count();

        if repetition_count >= 2 || board.halfmove_clock >= 100 {
            return 0;
        }

        let is_in_check = board.is_in_check(board.stm);

        if !is_in_check {
            let stand_pat_score = evaluator.evaluate(board);

            if stand_pat_score > beta {
                self.pruned_nodes += 1;
                return beta; // Fail high
            }
            alpha = max(alpha, stand_pat_score);
        }

        // Generate all moves if in check, otherwise use captures only
        let mut legal_moves = MoveBuffer::new();
        board.generate_legal_moves(&mut legal_moves, true);

        if is_in_check && legal_moves.is_empty() {
            // return Losing Mate score
            return adjust_score_for_ply(-MATE_SCORE, ply);
        }

        let seed = board.hash.wrapping_add(ply as u64);
        sort_moves(
            board,
            legal_moves.as_mut_slice(),
            &[None, None],
            None,
            &self.history,
            seed,
        );

        for mv in legal_moves {
            let mut board_copy = *board;
            if board_copy.make_move(mv).is_err() {
                continue;
            }

            self.hash_history.push(board_copy.hash);

            let score = -self.quiescence_search(&board_copy, ply + 1, -beta, -alpha, evaluator);

            self.hash_history.pop();

            if score >= beta {
                self.pruned_nodes += 1;
                return beta; // Fail high
            }
            alpha = max(alpha, score)
        }
        alpha
    }

    fn start(&mut self) {
        self.in_progress = true;
        if let Some(flag) = &self.search_running {
            flag.store(true, Ordering::Release);
        }
    }

    fn finish(&mut self) {
        self.in_progress = false;
        if let Some(flag) = &self.search_running {
            flag.store(false, Ordering::Release);
        }
    }

    fn should_stop(&self) -> bool {
        if let Some(flag) = &self.search_running
            && !flag.load(Ordering::Acquire)
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
    fn decay_history(&mut self) {
        trace!("Decaying history by 2");
        for from in 0..64 {
            for to in 0..64 {
                self.history[from][to] /= 2;
            }
        }
    }
}

// Adjusts Score to be relative to root.
// To be called before entry is stored in TranspositionTable
// Takes ply-dependent score and converts it to 'absolute' score
fn has_non_pawn_material(board: &Board) -> bool {
    let side = board.stm;
    let side_pieces = board.positions.get_side_bb(side);
    let pawns = board.positions.get_piece_bb(side, Piece::Pawn);
    let king = board.positions.get_piece_bb(side, Piece::King);
    (*side_pieces & !(*pawns | *king)).any()
}

// Adjusts Score to encode mate distance in the score
// Takes ply-independent score and converts it to also hold ply info
fn adjust_score_for_ply(score: i32, ply: usize) -> i32 {
    if score == i32::MIN {
        debug_assert!(false, "BUG: adjust_score_for_ply called with i32::MIN");
        error!("BUG: adjust_score_for_ply called with i32::MIN");
        return -MATE_SCORE;
    }
    if score.abs() > MATE_THRESHOLD {
        if score > 0 {
            score.saturating_sub(ply as i32)
        } else {
            score.saturating_add(ply as i32)
        }
    } else {
        score
    }
}

fn adjust_score_from_ply(score: i32, ply: usize) -> i32 {
    if score == i32::MIN {
        debug_assert!(false, "BUG: adjust_score_from_ply called with i32::MIN");
        error!("BUG: adjust_score_from_ply called with i32::MIN");
        return -MATE_SCORE;
    }
    if score.abs() > MATE_THRESHOLD {
        if score > 0 {
            score.saturating_add(ply as i32)
        } else {
            score.saturating_sub(ply as i32)
        }
    } else {
        score
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::log::init;

    use super::*;

    #[test]
    fn test_null_move_pruning() {
        init();
        let _ = utils::log::toggle_file_logging(true);
        let mut search_with_null = Search::new(10);
        let mut search_without_null = Search::new(10);

        assert!(search_without_null.enable_nmp);
        search_without_null.toggle_nmp();
        assert!(!search_without_null.enable_nmp);

        let board = Board::from_fen(KIWIPETE);
        println!("{board}");
        let evaluator = CompositeEvaluator::balanced();

        info!("Starting with null move pruning");
        let start = std::time::Instant::now();
        let result_with = search_with_null.find_best_move(&board, &evaluator);
        let time_with = start.elapsed();

        info!("Starting without null move pruning");
        let start = std::time::Instant::now();
        let result_without = search_without_null.find_best_move(&board, &evaluator);
        let time_without = start.elapsed();

        println!(
            "With null move: {} nodes in {:?}",
            result_with.nodes_searched, time_with
        );
        println!(
            "Without null move: {} nodes in {:?}",
            result_without.nodes_searched, time_without
        );

        assert_ne!(
            result_with.nodes_searched, result_without.nodes_searched,
            "Node counts should be different"
        );
        assert!(
            result_with.nodes_searched < result_without.nodes_searched,
            "NMP should reduce node count"
        );
    }
}
