//! MiniMax Search with Alpha-Beta pruning and Iterative Deepening
//! Also implements various standard techniques like:
//! - Late Move Reduction
//! - Null Move Pruning
//! - Aspiration Windows

use std::cmp::{max, min};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tracing::trace_span;

use crate::moves::move_gen::{CapturesOnly, generate_legal_moves};
use crate::prelude::*;
use crate::search::move_ordering::{MainSearchPolicy, MoveScoringPolicy, sort_moves};
use crate::search::move_picker::MovePicker;
use crate::search::tt::{ScoreTypes, TranspositionEntry, TranspositionTable};
use crate::search::{SearchEngine, SearchResult, SearchStats, common::*};

/// Consts
const ASP_START_WINDOW: i32 = 48;
const ASP_MAX_WINDOW: i32 = 4096;
const HISTORY_SIZE: usize = 128;
const DELTA_MARGIN: i32 = 700;
const SEE_THRESHOLD: i32 = -100;

/// Holds pv_node and curr ply
#[derive(Clone, Copy)]
pub struct SearchContext {
    ply: usize,
    is_pv_node: bool,
}

impl SearchContext {
    pub fn root() -> Self {
        Self {
            ply: 0,
            is_pv_node: true,
        }
    }

    pub fn new_child(&self, is_pv_child: bool) -> Self {
        SearchContext {
            ply: self.ply + 1,
            is_pv_node: is_pv_child,
        }
    }
}

/// Struct that holds relatively large tables
#[derive(Debug)]
pub struct SearchTables {
    killer_moves: [[Option<Move>; 2]; MAX_PLY],
    history: [[i32; NUM_SQUARES]; NUM_SQUARES],
}

impl Default for SearchTables {
    fn default() -> Self {
        Self {
            killer_moves: [[None; 2]; MAX_PLY],
            history: [[0; NUM_SQUARES]; NUM_SQUARES],
        }
    }
}

impl SearchTables {
    fn new() -> Self {
        Self {
            killer_moves: [[None; 2]; MAX_PLY],
            history: [[0; NUM_SQUARES]; NUM_SQUARES],
        }
    }

    /// Clears `killer_moves`.
    /// `history` is not cleared as it persists across searches
    fn clear(&mut self) {
        self.killer_moves = [[None; 2]; MAX_PLY]
    }

    /// Update `killer_moves`
    /// Stores given `Move` in `killer_moves[ply][0]` while backing up
    /// the prev occupant to `killer_moves[ply][1]`
    fn update_killers(&mut self, ply: usize, mv: Move) {
        if ply < MAX_PLY {
            self.killer_moves[ply][1] = self.killer_moves[ply][0];
            self.killer_moves[ply][0] = Some(mv);
        }
    }

    /// Update `history` table.
    /// Indexes as `history[mv.from][mv.to]`
    /// Scores are stored as `depth ^ 2`
    fn update_history(&mut self, mv: Move, depth: u16) {
        let from = mv.from_idx() as usize;
        let to = mv.to_idx() as usize;
        self.history[from][to] += depth as i32 * depth as i32;
    }

    /// Decay's all elements in `history` by dividing it by 2
    fn decay_history(&mut self) {
        for from in 0..64 {
            for to in 0..64 {
                self.history[from][to] /= 2;
            }
        }
    }
}

#[derive(Debug)]
pub struct RepetitionTable {
    hashes: [u64; HISTORY_SIZE],
    len: usize,
}

impl Default for RepetitionTable {
    fn default() -> Self {
        Self {
            hashes: [0; HISTORY_SIZE],
            len: Default::default(),
        }
    }
}

impl RepetitionTable {
    fn new() -> Self {
        Self {
            hashes: [0; HISTORY_SIZE],
            len: 0,
        }
    }

    #[inline]
    fn push(&mut self, hash: u64) {
        debug_assert!(
            self.len < HISTORY_SIZE,
            "RepetitionTable overflow! len={}, max={}",
            self.len,
            HISTORY_SIZE
        );
        if self.len < HISTORY_SIZE {
            self.hashes[self.len] = hash;
            self.len += 1;
        }
    }

    #[inline]
    fn pop(&mut self) {
        if self.len > 0 {
            self.len -= 1;
        }
    }

    #[inline]
    fn count_repetitions(&self, hash: u64) -> usize {
        self.hashes[..self.len]
            .iter()
            .filter(|&&h| h == hash)
            .count()
    }

    #[inline]
    fn clear(&mut self) {
        self.len = 0;
    }
}

#[derive(Debug)]
pub struct AlphaBetaSearch {
    /// Core search data
    nodes_searched: u64,
    search_cycle: u8,
    /// Search params
    config: SearchConfig,
    limits: SearchLimits,
    /// External deps
    evaluator: Box<dyn Evaluator>,
    search_running: Option<Arc<AtomicBool>>,
    /// Move ordering & history
    search_tables: Box<SearchTables>,
    /// Transposition table
    tt: TranspositionTable,
    /// Repetition detection
    pub repetition_table: RepetitionTable,
    /// Status
    in_progress: bool,
    start_time: Instant,
    /// Debug/tuning
    stats: SearchStats,
}

impl Default for AlphaBetaSearch {
    fn default() -> Self {
        Self {
            nodes_searched: Default::default(),
            search_cycle: Default::default(),
            config: Default::default(),
            limits: Default::default(),
            evaluator: Box::new(CompositeEvaluator::default()),
            search_running: Default::default(),
            search_tables: Default::default(),
            tt: Default::default(),
            repetition_table: Default::default(),
            in_progress: Default::default(),
            start_time: Instant::now(),
            stats: SearchStats::default(),
        }
    }
}

impl AlphaBetaSearch {
    pub fn new(evaluator: Box<dyn Evaluator>) -> Self {
        Self {
            config: SearchConfig::default(),
            limits: SearchLimits::default(),
            nodes_searched: 0,
            search_cycle: 0,
            start_time: Instant::now(),
            in_progress: false,
            evaluator,
            tt: TranspositionTable::new(16),
            search_tables: Box::new(SearchTables::new()),
            repetition_table: RepetitionTable::new(),
            search_running: None,
            stats: SearchStats::new(),
        }
    }

    /// Constructor to control various techniques in search, for ex,
    /// enable ASP, disable NMP, etc.
    pub fn with_config(mut self, config: SearchConfig) -> miette::Result<Self> {
        if self.config.hash_size_mb != config.hash_size_mb {
            self.tt.change_size(config.hash_size_mb)?;
        }
        self.config = config;
        Ok(self)
    }

    pub fn set_evaluator(&mut self, evaluator: Box<dyn Evaluator>) -> miette::Result<()> {
        miette::ensure!(
            !self.in_progress,
            "Cannot change Eval while search in progress"
        );
        self.evaluator = evaluator;
        Ok(())
    }

    /// Constructor to set limits for search. Time, node count, depth
    pub fn with_limits(mut self, limits: SearchLimits) -> Self {
        self.limits = limits;
        self
    }
}

impl SearchEngine for AlphaBetaSearch {
    type Output = AlphaBetaSearch;

    fn init(self, search_running: Arc<AtomicBool>) -> Self::Output {
        trace!("AlphaBeta: Initialized");
        Self {
            search_running: Some(search_running),
            ..self
        }
    }

    fn search(&mut self, board: &Board) -> SearchResult {
        self.find_best_move(board)
    }

    fn set_depth(&mut self, depth: u16) {
        self.limits.max_depth = Some(depth)
    }

    fn set_time(&mut self, time_ms: u64) {
        self.limits.max_time = Some(Duration::from_millis(time_ms))
    }

    fn set_nodes(&mut self, nodes: u64) {
        self.limits.max_nodes = Some(nodes)
    }

    fn clear(&mut self) {
        self.tt.clear();
        self.search_cycle = 0;
        self.repetition_table.clear();
        self.search_tables.clear();
        self.stats = SearchStats::new();
    }

    fn stop(&mut self) {
        if self.in_progress {
            self.in_progress = true;
        }
    }

    fn get_stats(&mut self) -> SearchStats {
        self.stats.nodes_searched = self.nodes_searched;
        self.stats.time_elapsed = self.start_time.elapsed();
        self.stats.hash_full = self.tt.hash_full();
        self.stats.calculate_nps();
        self.stats
    }

    fn clone_engine(&self) -> Box<dyn SearchEngine<Output = Self::Output>> {
        todo!()
    }

    fn get_config(&self) -> SearchConfig {
        self.config
    }

    fn get_limits(&self) -> SearchLimits {
        self.limits
    }
}

// Main search
impl AlphaBetaSearch {
    pub fn find_best_move(&mut self, board: &Board) -> SearchResult {
        self.start();

        let span = trace_span!("search_root");
        let _guard = span.enter();

        debug!(
            "Finding best move for board: '{:?}' with max_depth: {:?}, max_time: {:?}",
            board.to_fen(),
            self.limits.max_depth,
            self.limits.max_time
        );

        self.prepare_for_search();
        self.start_time = Instant::now();
        self.repetition_table.push(board.hash);
        self.search_cycle = self.search_cycle.wrapping_add(1);

        let mut legal_moves = MoveBuffer::new();
        board.generate_legal_moves(&mut legal_moves, false);

        if legal_moves.is_empty() {
            debug!("No legal moves");
            let in_check = board.is_in_check(board.stm);
            let mut mate_in = None;
            let score = if in_check {
                mate_in = Some(0);
                -MATE_SCORE
            } else {
                STALEMATE_SCORE
            };
            self.repetition_table.pop();
            self.finish();
            return SearchResult {
                best_move: None,
                score,
                depth: 0,
                nodes_searched: self.nodes_searched,
                time_taken: self.start_time.elapsed(),
                pv: None,
                is_mate: in_check,
                mate_in,
            };
        }

        // Initialized to first move as fallback
        let mut best_move = legal_moves.first().copied();
        let mut best_score = i32::MIN + 1;
        let mut completed_depth = u16::default();

        // Prev score for Aspiration Windows
        let mut prev_score = 0;

        'id_loop: for depth in 1..=self.limits.max_depth.unwrap_or(MAX_PLY as u16) {
            if self.should_stop() {
                break 'id_loop;
            }

            debug!("Iterative Deepening current depth: {depth}");

            let (local_best_move, local_best_score) = self.root_search_with_aspiration(
                board,
                depth,
                &mut legal_moves,
                best_move,
                prev_score,
            );

            if self.should_stop() {
                break 'id_loop;
            }

            completed_depth = depth;
            best_move = local_best_move;
            best_score = local_best_score;
            prev_score = best_score;

            if std::hint::likely(self.config.emit_info) {
                self.emit_info_string(depth, best_score, best_move);
            }
        }

        self.repetition_table.pop();
        if std::hint::unlikely(self.config.collect_stats) {
            self.stats.depth_reached = completed_depth;
            self.get_stats().log_summary();
        }
        self.finish();
        SearchResult {
            best_move,
            score: best_score,
            depth: completed_depth,
            nodes_searched: self.nodes_searched,
            time_taken: self.start_time.elapsed(),
            pv: Some(vec![
                best_move.expect("Already been initialized to first move"),
            ]),
            // NOTE: Figure out how to get this.
            is_mate: false,
            mate_in: None,
        }
    }

    fn alpha_beta(
        &mut self,
        board: &Board,
        context: SearchContext,
        depth: u16,
        mut alpha: i32,
        mut beta: i32,
    ) -> i32 {
        if self.should_stop() {
            // Neutral score because search was stopped
            return 0;
        }

        // Necessary to prevent search extensions from explosion
        if context.ply >= MAX_PLY {
            // Treat this as leaf node
            return board.evaluate_position(&*self.evaluator);
        }

        if depth == 0 {
            return self.quiescence_search(board, context, alpha, beta);
        }

        if alpha >= beta {
            error!("Invalid alpha-beta window: alpha {alpha}, beta: {beta}");
            panic!("Invalid alpha-beta window: alpha {alpha}, beta: {beta}");
        }

        // Every entry into this function is exploring a new node
        // Doesn't matter if this gets pruned away
        self.nodes_searched += 1;

        let ply = context.ply;
        let original_alpha = alpha;
        let is_in_check = board.is_in_check(board.stm);

        let current_hash = board.hash;
        if self.is_draw(board) {
            // debug!(
            //     "Draw! Halfmove clock: {}, Repetition count: {} for {}",
            //     board.halfmove_clock,
            //     self.repetition_table.count_repetitions(current_hash),
            //     current_hash
            // );
            if self.config.collect_stats {
                self.stats.draw_returns += 1;
            }
            return 0;
        }

        let mut tt_move = Move::default();

        // TT Probe
        if self.config.collect_stats {
            self.stats.tt_probes += 1;
        }
        if let Some(entry) = self.tt.probe(current_hash)
            && entry.matches(current_hash)
        {
            if self.config.collect_stats {
                self.stats.tt_hits += 1;
            }
            tt_move = entry.get_best_move();
            if entry.get_depth() >= depth {
                let score = adjust_score_for_ply(entry.get_score(), ply);

                match entry.get_score_type() {
                    ScoreTypes::Exact => {
                        // Exact scores returned as is
                        if self.config.collect_stats {
                            self.stats.tt_cutoffs += 1;
                            self.stats.tt_exact_returns += 1;
                        }
                        return score;
                    }
                    ScoreTypes::LowerBound => {
                        // The true score is 'at least' prev score
                        // if this is enough to beat beta, this can be pruned
                        alpha = max(alpha, score);
                        if alpha >= beta {
                            if self.config.collect_stats {
                                self.stats.tt_cutoffs += 1;
                            }
                            return beta;
                        }
                    }
                    ScoreTypes::UpperBound => {
                        // The true score is 'at most' prev score
                        // If this is enough to beat beta, this can be pruned
                        beta = min(beta, score);
                        if alpha >= beta {
                            if self.config.collect_stats {
                                self.stats.tt_cutoffs += 1;
                            }
                            return alpha;
                        }
                    }
                }
            }
        }

        if alpha >= beta {
            panic!("Invalid alpha-beta window: alpha: {alpha}, beta: {beta}");
        }

        // Null Move Pruning
        let child_context = context.new_child(false);
        if let Some(nmp_score) = self.try_null_move_pruning(board, child_context, depth, beta) {
            return nmp_score;
        }

        let mut legal_moves = MoveBuffer::new();
        board.generate_legal_moves(&mut legal_moves, false);

        if legal_moves.is_empty() {
            if self.config.collect_stats {
                self.stats.mate_returns += 1;
            }
            return if board.is_in_check(board.stm) {
                -MATE_SCORE + ply as i32
            } else {
                STALEMATE_SCORE
            };
        }

        // Node that made through pruning, will be actually searched
        if self.config.collect_stats {
            self.stats.main_search_nodes += 1;
        }

        let mut best_move_this_node = legal_moves
            .first()
            .copied()
            .expect("Atleast one move should exist in the buffer");
        let mut best_score = i32::MIN + 1;

        let mut picker = MovePicker::new(
            board,
            legal_moves.as_mut_slice(),
            &self.search_tables.killer_moves[ply],
            Some(tt_move),
            &self.search_tables.history,
        );

        let mut move_index = 0;

        while let Some(mv) = picker.next_best() {
            // Use Unmake_move instead
            let mut board_copy = *board;
            board_copy.make_move(mv).expect("Move is already legal");

            self.repetition_table.push(board_copy.hash);

            let mut score: i32;

            let is_pv_node = move_index == 0;
            let child_is_pv = context.is_pv_node && is_pv_node;

            // Starts as non-PV unless the parent is PV and this is the first move
            let mut child_context = context.new_child(child_is_pv);

            let mut extension = 0;
            let move_gives_check = board_copy.is_in_check(board_copy.stm);

            if is_in_check {
                extension = 1;
            } else {
                // Passed pawn extension
                if let Some(piece) = board.get_piece_at(mv.from_sq())
                    && piece == Piece::Pawn
                {
                    let rank = mv.to_sq().row();
                    let is_threatening_promo = if board.stm == Side::White {
                        rank == 6
                    } else {
                        rank == 1
                    };

                    if is_threatening_promo {
                        extension = 1
                    }
                }
            }

            let new_depth = depth + extension;

            if is_pv_node {
                score = self.pv_search(&board_copy, child_context, new_depth, alpha, beta);
            } else if self.should_reduce(depth, move_index, mv, is_in_check, move_gives_check) {
                let mut reduction = self.lmr_reduction(depth, move_index);

                if context.is_pv_node {
                    reduction = reduction.saturating_sub(1);
                }
                reduction = reduction.min(depth - 1);

                let reduced_depth = (depth - 1).saturating_sub(reduction);

                if self.config.collect_stats {
                    self.stats.lmr_attempts += 1;
                }

                score = -self.alpha_beta(
                    &board_copy,
                    child_context,
                    reduced_depth,
                    -alpha - 1,
                    -alpha,
                );

                if score > alpha {
                    // Since LMR failed high, this node should now be PV node
                    if self.config.collect_stats {
                        self.stats.lmr_research += 1;
                    }
                    child_context.is_pv_node = true;
                    score = self.zw_search(&board_copy, child_context, new_depth, alpha, beta);
                }
            } else {
                score = self.zw_search(&board_copy, child_context, new_depth, alpha, beta);
            }

            self.repetition_table.pop();

            if self.should_stop() {
                return 0;
            }

            if score > best_score {
                best_score = score;
                best_move_this_node = mv;
            }

            alpha = max(alpha, score);

            if alpha >= beta {
                if self.config.collect_stats {
                    if move_index < MAX_PLY {
                        self.stats.cutoff_at_move[move_index] += 1;
                    }
                    self.stats.beta_cutoffs_main += 1;
                    self.stats.pruned_nodes += 1;
                }

                // This is beta-cutoff (Fail high)
                // If move is quiet, this is a good candidate for killer moves
                if !mv.is_capture() && ply < MAX_PLY {
                    self.search_tables.update_killers(ply, mv);
                    self.search_tables.update_history(mv, depth);
                }

                let entry_to_store = TranspositionEntry::new(
                    current_hash,
                    best_move_this_node,
                    adjust_score_from_ply(beta, ply),
                    depth as u8,
                    ScoreTypes::LowerBound,
                    self.search_cycle,
                );

                self.tt.store(entry_to_store);

                return beta;
            }
            move_index += 1;
        }

        if best_score == i32::MIN + 1 {
            error!(
                "Mad cooked: alpha: {alpha}, beta: {beta}, best_score: {best_score}, legal_moves searched: {move_index}",
            );
        }

        let score_type = if best_score <= original_alpha {
            // We failed to raise alpha. This is a fail-low.
            // The score is an upper bound on the node's true value
            if self.config.collect_stats {
                self.stats.fail_lows += 1;
            }
            ScoreTypes::UpperBound
        } else {
            // Successfully raised alpha but did not fail high.
            // This means the exact best score was found in the (alpha, beta) window.
            if self.config.collect_stats {
                self.stats.exact_scores += 1;
            }
            ScoreTypes::Exact
        };

        let entry_to_store = TranspositionEntry::new(
            current_hash,
            best_move_this_node,
            adjust_score_from_ply(best_score, ply),
            depth as u8,
            score_type,
            self.search_cycle,
        );
        self.tt.store(entry_to_store);

        alpha
    }

    fn quiescence_search(
        &mut self,
        board: &Board,
        context: SearchContext,
        mut alpha: i32,
        beta: i32,
    ) -> i32 {
        if self.should_stop() {
            return 0;
        }

        if context.ply > MAX_PLY {
            return board.evaluate_position(&*self.evaluator);
        }

        if self.should_stop() {
            return 0;
        }

        // Every entry into this function is exploring a new node
        // Doesn't matter if this gets pruned away
        self.nodes_searched += 1;

        if self.is_draw(board) {
            if self.config.collect_stats {
                self.stats.draw_returns += 1;
            }
            return 0;
        }

        let is_in_check = board.is_in_check(board.stm);

        let stand_pat_score;

        if !is_in_check {
            stand_pat_score = board.evaluate_position(&*self.evaluator);

            if stand_pat_score >= beta {
                if self.config.collect_stats {
                    self.stats.standpat_returns += 1;
                    self.stats.pruned_nodes += 1;
                }
                // Fail high
                return beta;
            }
            alpha = max(alpha, stand_pat_score)
        } else {
            stand_pat_score = i32::MIN;
        }

        // Generate all moves in check, otherwise use forcing moves only
        let mut legal_moves = MoveBuffer::new();
        generate_legal_moves::<CapturesOnly>(board, &mut legal_moves);

        if is_in_check && legal_moves.is_empty() {
            // return Losing Mate score
            if self.config.collect_stats {
                self.stats.mate_returns += 1;
            }
            return adjust_score_for_ply(-MATE_SCORE, context.ply);
        }

        // Passed through early-exits
        // full-qsearch node
        if self.config.collect_stats {
            self.stats.qsearch_nodes += 1;
        }

        let mut picker = MovePicker::new_qsearch(board, legal_moves.as_mut_slice());

        while let Some(mv) = picker.next_best() {
            if !is_in_check {
                // Delta pruning
                if mv.is_capture() {
                    let captured_piece_value = if mv.is_enpassant() {
                        Piece::Pawn.victim_score()
                    } else if let Some(p) = board.get_piece_at(mv.to_sq()) {
                        p.victim_score()
                    } else {
                        unreachable!(
                            "If move is a capture, then it should either be enpassant or 'to' square should hold a piece"
                        )
                    };
                    if stand_pat_score
                        + captured_piece_value
                        + if mv.is_promotion() {
                            DELTA_MARGIN + 200
                        } else {
                            DELTA_MARGIN
                        }
                        < alpha
                    {
                        if self.config.collect_stats {
                            self.stats.delta_pruning_cutoffs += 1;
                            self.stats.pruned_nodes += 1;
                        }
                        continue; // Skip this node
                    }
                }
                // SEE pruning
                if board.static_exchange_evaluation(mv) < SEE_THRESHOLD {
                    if self.config.collect_stats {
                        self.stats.see_pruning_cutoffs += 1;
                        self.stats.pruned_nodes += 1;
                    }
                    continue;
                }
            }
            let mut board_copy = *board;
            if let Err(e) = board_copy.make_move(mv) {
                error!(
                    "Making move on board (fen: {:?}) failed with error: {}",
                    board_copy.to_fen(),
                    e
                );
                continue;
            }
            self.repetition_table.push(board_copy.hash);

            let child_context = context.new_child(context.is_pv_node);
            let score = -self.quiescence_search(&board_copy, child_context, -beta, -alpha);

            self.repetition_table.pop();

            if score >= beta {
                // Fail-High
                if self.config.collect_stats {
                    self.stats.beta_cutoffs_qs += 1;
                    self.stats.pruned_nodes += 1;
                }
                return beta;
            }
            alpha = max(alpha, score);
        }
        alpha
    }
}

impl AlphaBetaSearch {
    #[inline]
    fn should_stop(&self) -> bool {
        if let Some(flag) = &self.search_running
            && !flag.load(Ordering::Acquire)
        {
            debug!("Stop signal recieved");
            return true;
        }

        if !self.in_progress {
            debug!("`in_progress` is switched to false");
            return true;
        }

        if let Some(max_time) = self.limits.max_time
            && self.start_time.elapsed() >= max_time
        {
            debug!("Max time utilized");
            return true;
        }

        if self
            .limits
            .max_nodes
            .is_some_and(|l| self.nodes_searched >= l)
        {
            debug!("Node limit exhauseted");
            return true;
        }

        false
    }

    fn emit_info_string(&self, depth: u16, score: i32, best_move: Option<Move>) {
        let nps =
            (self.nodes_searched * 1000) / self.start_time.elapsed().as_millis().max(1) as u64;
        let best_move_uci = best_move.map(|m| m.uci()).unwrap_or_default();

        let msg = format!(
            "info depth {} score cp {} nodes {} nps {} pv {}",
            depth, score, self.nodes_searched, nps, best_move_uci
        );
        println!("{msg}");
        debug!(msg);
    }

    #[inline]
    fn start(&mut self) {
        self.in_progress = true;
        if let Some(flag) = &self.search_running {
            flag.store(true, Ordering::Relaxed);
        }
    }

    fn finish(&mut self) {
        self.in_progress = false;
        if let Some(flag) = &self.search_running {
            flag.store(false, Ordering::Relaxed);
        }
    }

    #[inline]
    fn is_draw(&self, board: &Board) -> bool {
        board.halfmove_clock >= 100 || self.repetition_table.count_repetitions(board.hash) >= 2
    }

    fn prepare_for_search(&mut self) {
        self.nodes_searched = 0;
        self.search_tables.clear();
        self.search_tables.decay_history();
        self.stats = SearchStats::new();
    }
}

impl AlphaBetaSearch {
    /// Full window search for the first move (PV-move)
    #[inline(always)]
    fn pv_search(
        &mut self,
        board: &Board,
        context: SearchContext,
        depth: u16,
        alpha: i32,
        beta: i32,
    ) -> i32 {
        -self.alpha_beta(board, context, depth - 1, -beta, -alpha)
    }

    fn root_search_with_aspiration(
        &mut self,
        board: &Board,
        depth: u16,
        legal_moves: &mut MoveBuffer,
        prev_best_move: Option<Move>,
        prev_score: i32,
    ) -> (Option<Move>, i32) {
        self.sort_moves::<MainSearchPolicy>(board, legal_moves, prev_best_move, depth as usize);

        let use_asp = self.config.enable_asp
            && depth > 1
            && prev_score.abs() < MATE_THRESHOLD - ASP_MAX_WINDOW;

        let mut window = ASP_START_WINDOW;

        let mut alpha_base = if use_asp {
            prev_score.saturating_sub(window)
        } else {
            i32::MIN + 1
        };

        let mut beta_base = if use_asp {
            prev_score.saturating_add(window)
        } else {
            i32::MAX
        };
        trace!("Using ASP: {use_asp}");

        let mut tries: usize = 0;
        loop {
            if self.should_stop() {
                return (legal_moves.first().copied(), i32::MIN + 1);
            }
            trace!("ASP window: ({alpha_base}, {beta_base})");

            let (best_move, best_score) =
                self.root_search_attempt(board, depth, alpha_base, beta_base, legal_moves);

            if !use_asp {
                return (best_move, best_score);
            }

            if tries >= 4 {
                debug!("Tried ASP 4 times, No-doy");
                return (best_move, best_score);
            }

            // - Asymmetric widening: Increase/decrease based on fail high/low
            if best_score <= alpha_base {
                // Fail Low
                if self.config.collect_stats {
                    self.stats.asp_fail_low += 1;
                    self.stats.asp_research += 1;
                }
                tries += 1;
                window = window.saturating_mul(2).min(ASP_MAX_WINDOW);
                alpha_base = prev_score.saturating_sub(window);
            } else if best_score >= beta_base {
                // Fail High
                if self.config.collect_stats {
                    self.stats.asp_fail_high += 1;
                    self.stats.asp_research += 1;
                }
                tries += 1;
                window = window.saturating_mul(2).min(ASP_MAX_WINDOW);
                beta_base = prev_score.saturating_add(window);
            } else {
                return (best_move, best_score);
            }
        }
    }

    fn root_search_attempt(
        &mut self,
        board: &Board,
        depth: u16,
        alpha_base: i32,
        beta_base: i32,
        legal_moves: &MoveBuffer,
    ) -> (Option<Move>, i32) {
        let mut alpha = alpha_base;
        let beta = beta_base;

        let mut local_best_move: Option<Move> = legal_moves.first().copied();
        let mut local_best_score: i32 = i32::MIN + 1;

        for &mv in legal_moves {
            if self.should_stop() {
                break;
            }

            let mut board_copy = *board;
            if let Err(e) = board_copy.make_move(mv) {
                error!(
                    "Making move on board (fen: {:?}) failed with error: {}",
                    board_copy.to_fen(),
                    e
                );
                continue;
            }

            self.repetition_table.push(board_copy.hash);

            let root_child_context = SearchContext {
                ply: 1,
                is_pv_node: true,
            };

            let score = -self.alpha_beta(&board_copy, root_child_context, depth - 1, -beta, -alpha);

            self.repetition_table.pop();

            if self.should_stop() {
                break;
            }

            if score > local_best_score {
                local_best_score = score;
                local_best_move = Some(mv);
            }

            alpha = max(alpha, local_best_score);

            if alpha >= beta {
                break;
            }
        }

        (local_best_move, local_best_score)
    }

    /// Zero-window search for non-PV moves, with re-search on fail-high
    #[inline(always)]
    fn zw_search(
        &mut self,
        board: &Board,
        context: SearchContext,
        depth: u16,
        alpha: i32,
        beta: i32,
    ) -> i32 {
        let mut score = -self.alpha_beta(board, context, depth - 1, -alpha - 1, -alpha);
        if score > alpha && score < beta {
            // if ZWS fails, re-search with full window and make this new PV
            let re_search_child_context = context.new_child(true);
            score = -self.alpha_beta(board, re_search_child_context, depth - 1, -beta, -alpha);
        }

        score
    }

    /// Null move pruning
    fn try_null_move_pruning(
        &mut self,
        board: &Board,
        context: SearchContext,
        depth: u16,
        beta: i32,
    ) -> Option<i32> {
        if !self.config.enable_nmp {
            return None;
        }

        if depth < 5
            || context.ply == 0
            || board.is_in_check(board.stm)
            || !has_non_pawn_material(board)
        {
            return None;
        }

        let null_reduction = if depth >= 6 { 4 } else { 2 };
        let null_depth = depth.saturating_sub(null_reduction);

        if self.config.collect_stats {
            self.stats.null_move_attempts += 1;
        }

        let mut null_board = *board;
        null_board.make_null_move();

        let child_context = context.new_child(false);
        let score = -self.alpha_beta(&null_board, child_context, null_depth, -beta, -beta + 1);

        if score >= beta {
            if self.config.collect_stats {
                self.stats.null_move_cutoffs += 1;
                self.stats.pruned_nodes += 1;
            }
            Some(beta)
        } else {
            None
        }
    }

    /// Late Move Reduction
    #[inline]
    fn lmr_reduction(&self, depth: u16, move_index: usize) -> u16 {
        let base = 0.20 + ((depth as f32).ln() * (move_index as f32).ln()) / 3.35;
        (base as u16).min(depth - 1)
    }

    fn sort_moves<P: MoveScoringPolicy>(
        &self,
        board: &Board,
        legal_moves: &mut MoveBuffer,
        hint: Option<Move>,
        depth: usize,
    ) {
        let seed = board.hash.wrapping_add(depth as u64);
        sort_moves::<P>(
            board,
            legal_moves.as_mut_slice(),
            &self.search_tables.killer_moves[depth],
            hint,
            &self.search_tables.history,
            seed,
        );
    }

    /// Check if lmr should be applied
    #[inline]
    fn should_reduce(
        &self,
        depth: u16,
        move_index: usize,
        mv: Move,
        in_check: bool,
        gives_check: bool,
    ) -> bool {
        self.config.enable_lmr
            && depth >= 3
            && move_index >= 3
            && !mv.is_capture()
            && !mv.is_promotion()
            && !in_check
            && !gives_check
    }
}
