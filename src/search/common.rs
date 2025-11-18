use tracing::debug_span;

use crate::prelude::*;
use std::{ops::Add, time::Duration};

/// Common statistics for all search types
#[derive(Debug, Clone)]
pub struct SearchStats {
    // Basic stats
    pub nodes_searched: u64, // Total nodes including qsearch
    pub depth_reached: u8,
    pub time_elapsed: Duration,
    pub nps: u64,
    pub hash_full: u16, // per-mille

    // Node type
    pub main_search_nodes: u64, // Only main search nodes (excludes qsearch)
    pub qsearch_nodes: u64,     // Quiescence search nodes

    // Early exit tracking
    pub tt_exact_returns: u64, // Returned exact score from TT
    pub draw_returns: u64,     // Returned due to draw detection
    pub mate_returns: u64,     // Returned due to mate/stalemate score
    pub standpat_returns: u64, // Returned due to Stand-pat cut-off in qsearch

    // Pruning stats
    pub pruned_nodes: u64, // Total nodes pruned

    // Transposition table stats
    pub tt_probes: u64,  // TT probe attempts
    pub tt_hits: u64,    // TT probes that found an entry
    pub tt_cutoffs: u64, // Times TT caused a cutoff (LowerBound / UpperBound)

    // Pruning techniques stats
    pub null_move_attempts: u64, // Times null move pruning was tried
    pub null_move_cutoffs: u64,  // Times null move pruning succeeded
    pub lmr_attempts: u64,       // Times LMR was attempted
    pub lmr_research: u64,       // Times LMR failed high and re-search was needed

    // QSearch Pruning
    pub delta_pruning_cutoffs: u64, // Times delta pruning succeeded
    pub see_pruning_cutoffs: u64,   // Times SEE pruning helped

    // Aspiration windows
    pub asp_fail_high: u64, // Times aspiration window failed high
    pub asp_fail_low: u64,  // Times aspiration window failed low
    pub asp_research: u64,  // Total re-searches due to ASP

    // Alpha-Beta window
    pub beta_cutoffs_main: u64, // Times alpha >= beta (fail-high) in main search
    pub beta_cutoffs_qs: u64,   // Times alpha >= beta (fail-high) in qsearch
    pub exact_scores: u64,      // Times an exact score was found
    pub fail_lows: u64,         // Times we failed to raise alpha (fail-low)

    // Move ordering stats (CutOffStats)
    pub cutoff_at_move: [u64; MAX_PLY],
}

impl Default for SearchStats {
    fn default() -> Self {
        Self {
            nodes_searched: Default::default(),
            depth_reached: Default::default(),
            time_elapsed: Default::default(),
            nps: Default::default(),
            hash_full: Default::default(),
            pruned_nodes: Default::default(),
            qsearch_nodes: Default::default(),
            tt_probes: Default::default(),
            tt_hits: Default::default(),
            tt_cutoffs: Default::default(),
            null_move_cutoffs: Default::default(),
            beta_cutoffs_main: Default::default(),
            beta_cutoffs_qs: Default::default(),
            fail_lows: Default::default(),
            main_search_nodes: Default::default(),
            cutoff_at_move: [Default::default(); MAX_PLY],
            exact_scores: Default::default(),
            tt_exact_returns: Default::default(),
            draw_returns: Default::default(),
            mate_returns: Default::default(),
            standpat_returns: Default::default(),
            null_move_attempts: Default::default(),
            lmr_attempts: Default::default(),
            lmr_research: Default::default(),
            asp_fail_high: Default::default(),
            asp_fail_low: Default::default(),
            asp_research: Default::default(),
            delta_pruning_cutoffs: Default::default(),
            see_pruning_cutoffs: Default::default(),
        }
    }
}

impl Add for SearchStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut combined_cutoff_at_move = [0u64; MAX_PLY];
        for i in 0..MAX_PLY {
            combined_cutoff_at_move[i] = self.cutoff_at_move[i] + rhs.cutoff_at_move[i];
        }

        let total_nodes = self.nodes_searched + rhs.nodes_searched;
        let total_time = self.time_elapsed + rhs.time_elapsed;
        let time_ms = total_time.as_millis().max(1) as u64;
        let new_nps = (total_nodes * 1000) / time_ms;

        Self {
            nodes_searched: total_nodes,
            depth_reached: self.depth_reached.max(rhs.depth_reached),
            time_elapsed: total_time,
            nps: new_nps,
            hash_full: rhs.hash_full,

            main_search_nodes: self.main_search_nodes + rhs.main_search_nodes,
            qsearch_nodes: self.qsearch_nodes + rhs.qsearch_nodes,

            tt_exact_returns: self.tt_exact_returns + rhs.tt_exact_returns,
            draw_returns: self.draw_returns + rhs.draw_returns,
            mate_returns: self.mate_returns + rhs.mate_returns,
            standpat_returns: self.standpat_returns + rhs.standpat_returns,

            pruned_nodes: self.pruned_nodes + rhs.pruned_nodes,

            tt_probes: self.tt_probes + rhs.tt_probes,
            tt_hits: self.tt_hits + rhs.tt_hits,
            tt_cutoffs: self.tt_cutoffs + rhs.tt_cutoffs,

            null_move_attempts: self.null_move_attempts + rhs.null_move_attempts,
            null_move_cutoffs: self.null_move_cutoffs + rhs.null_move_cutoffs,
            lmr_attempts: self.lmr_attempts + rhs.lmr_attempts,
            lmr_research: self.lmr_research + rhs.lmr_research,

            delta_pruning_cutoffs: self.delta_pruning_cutoffs + rhs.delta_pruning_cutoffs,
            see_pruning_cutoffs: self.see_pruning_cutoffs + rhs.see_pruning_cutoffs,

            asp_fail_high: self.asp_fail_high + rhs.asp_fail_high,
            asp_fail_low: self.asp_fail_low + rhs.asp_fail_low,
            asp_research: self.asp_research + rhs.asp_research,

            beta_cutoffs_main: self.beta_cutoffs_main + rhs.beta_cutoffs_main,
            beta_cutoffs_qs: self.beta_cutoffs_qs + rhs.beta_cutoffs_qs,
            exact_scores: self.exact_scores + rhs.exact_scores,
            fail_lows: self.fail_lows + rhs.fail_lows,

            cutoff_at_move: combined_cutoff_at_move,
        }
    }
}

impl SearchStats {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    fn percent(numerator: u64, denominator: u64) -> f64 {
        if denominator == 0 {
            0.0
        } else {
            100.0 * numerator as f64 / denominator as f64
        }
    }

    pub fn calculate_nps(&mut self) {
        let time_ms = self.time_elapsed.as_millis().max(1) as u64;
        self.nps = (self.nodes_searched * 1000) / time_ms;
    }

    pub fn avg_cutoff_index(&self) -> f64 {
        let total_cutoffs: u64 = self.cutoff_at_move.iter().sum();
        if total_cutoffs == 0 {
            0.0
        } else {
            let weighted_sum: u64 = self
                .cutoff_at_move
                .iter()
                .enumerate()
                .map(|(i, &count)| i as u64 * count)
                .sum();
            weighted_sum as f64 / total_cutoffs as f64
        }
    }

    pub fn log_summary(&self) {
        let _span = debug_span!("search_stats").entered();
        debug!("=> SEARCH STATISTICS (depth {})", self.depth_reached);
        debug!(
            "NODES total={} time={:?} nps={}",
            self.nodes_searched, self.time_elapsed, self.nps
        );

        // Node accounting
        let early_exits =
            self.tt_cutoffs + self.draw_returns + self.null_move_cutoffs + self.standpat_returns;
        let terminal_nodes = self.mate_returns;
        let nodes_that_searched_moves = self.main_search_nodes + self.qsearch_nodes;
        let accounted = early_exits + terminal_nodes + nodes_that_searched_moves;
        let unaccounted = self.nodes_searched.saturating_sub(accounted);

        debug!("");
        debug!("==> Node Breakdown");
        debug!(
            "  - Early Exits:      {:>9} ({:>6.2}%)",
            early_exits,
            Self::percent(early_exits, self.nodes_searched)
        );
        debug!(
            "  - Terminal Nodes:   {:>9} ({:>6.2}%)",
            terminal_nodes,
            Self::percent(terminal_nodes, self.nodes_searched)
        );
        debug!(
            "  - Searched Moves:   {:>9} ({:>6.2}%)",
            nodes_that_searched_moves,
            Self::percent(nodes_that_searched_moves, self.nodes_searched)
        );
        if unaccounted > 0 {
            debug!(
                "  - Unaccounted:      {:>9} ({:>6.2}%)",
                unaccounted,
                Self::percent(unaccounted, self.nodes_searched)
            );
        }

        // Main Search Analysis
        debug!("");
        debug!("==> Main Search ({} nodes)", self.main_search_nodes);
        debug!(
            "  - Beta Cutoffs:     {:>9} ({:>6.2}%)",
            self.beta_cutoffs_main,
            Self::percent(self.beta_cutoffs_main, self.main_search_nodes)
        );
        debug!("  - Exact Scores:     {:>9}", self.exact_scores);
        debug!("  - Fail Lows:        {:>9}", self.fail_lows);

        // QSearch Analysis
        debug!("");
        debug!("==> QSearch ({} nodes)", self.qsearch_nodes);
        debug!(
            "  - Beta Cutoffs:      {:>9} ({:>6.2}%)",
            self.beta_cutoffs_qs,
            Self::percent(self.beta_cutoffs_qs, self.qsearch_nodes)
        );
        debug!("  - Delta Pruned:      {:>9}", self.delta_pruning_cutoffs);
        debug!("  - SEE Pruned:        {:>9}", self.see_pruning_cutoffs);

        // Pruning Techniques
        debug!("");
        debug!("==> Pruning & TT");
        debug!(
            "  - TT Hits:          {:>9} ({:>6.2}% of probes), hash_full: {}/1000",
            self.tt_hits,
            Self::percent(self.tt_hits, self.tt_probes),
            self.hash_full
        );
        debug!(
            "    - TT Cutoffs:     {:>9} ({:>6.2}% of hits)",
            self.tt_cutoffs,
            Self::percent(self.tt_cutoffs, self.tt_hits)
        );
        debug!("  - NMP Attempts:     {:>9}", self.null_move_attempts);
        debug!(
            "    - NMP Cutoffs:    {:>9} ({:>6.2}% success rate)",
            self.null_move_cutoffs,
            Self::percent(self.null_move_cutoffs, self.null_move_attempts)
        );
        debug!("  - LMR Attempts:     {:>9}", self.lmr_attempts);
        debug!(
            "    - LMR Researches: {:>9} ({:>6.2}% research rate)",
            self.lmr_research,
            Self::percent(self.lmr_research, self.lmr_attempts)
        );
        if self.asp_research > 0 {
            debug!(
                "  - ASP Researches:   {:>9} (high: {}, low: {})",
                self.asp_research, self.asp_fail_high, self.asp_fail_low
            );
        }

        // Move Ordering
        let total_cutoffs: u64 = self.cutoff_at_move.iter().sum();
        if total_cutoffs > 0 {
            debug!("");
            debug!("==> Move Ordering");
            debug!("  - Total Beta Cutoffs: {}", total_cutoffs);
            debug!("  - Avg. Cutoff Index:  {:.2}", self.avg_cutoff_index());

            let histogram: Vec<String> = self
                .cutoff_at_move
                .iter()
                .take(10) // Limit to first 10 for readability
                .enumerate()
                .filter(|&(_, &count)| count > 0)
                .map(|(i, count)| format!("{}:{}", i, count))
                .collect();

            if !histogram.is_empty() {
                debug!(
                    "  - Cutoff Histogram (move index:count): [{}]",
                    histogram.join(", ")
                );
            }
        }
    }
}

/// Configuration for search behavior
#[derive(Debug, Clone, Copy)]
pub struct SearchConfig {
    pub enable_nmp: bool,
    pub enable_asp: bool,
    pub enable_lmr: bool,
    pub emit_info: bool,
    pub collect_stats: bool, // TODO: feature-gate this
    pub hash_size_mb: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            enable_nmp: true,
            enable_asp: true,
            enable_lmr: true,
            emit_info: true,
            collect_stats: true, // Disabled for perf
            hash_size_mb: 16,
        }
    }
}

/// Search limits (time, depth, nodes)
#[derive(Default, Debug, Clone, Copy)]
pub struct SearchLimits {
    pub max_depth: Option<u8>,
    pub max_time: Option<Duration>,
    pub max_nodes: Option<u64>,
    pub mate_depth: Option<u8>,
}

impl SearchLimits {
    pub fn depth(depth: u8) -> Self {
        Self {
            max_depth: Some(depth),
            ..Default::default()
        }
    }

    pub fn time(time_ms: u64) -> Self {
        Self {
            max_time: Some(Duration::from_millis(time_ms)),
            ..Default::default()
        }
    }

    pub fn nodes(nodes: u64) -> Self {
        Self {
            max_nodes: Some(nodes),
            ..Default::default()
        }
    }

    pub fn infinite() -> Self {
        Self::default()
    }
}

/// Result of a search
#[derive(Debug, Default, Clone)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub nodes_searched: u64,
    pub time_taken: Duration,
    pub pv: Option<Vec<Move>>,
    pub is_mate: bool,
    pub mate_in: Option<i32>,
}

impl SearchResult {
    pub fn nps(&self) -> u64 {
        let time_ms = self.time_taken.as_millis().max(1) as u64;
        (self.nodes_searched * 1000) / time_ms
    }
}

/// Holds pv_node and curr ply
#[derive(Clone, Copy)]
pub struct SearchContext {
    pub ply: usize,
    pub is_pv_node: bool,
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

/// Helper functions for score adjustment
/// Adjusts Score to encode mate distance in the score
/// Takes ply-independent score and converts it to also hold ply info
#[inline(always)]
pub fn adjust_score_for_ply(score: i32, ply: usize) -> i32 {
    if score == i32::MIN {
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

// Adjusts Score to be relative to root.
// To be called before entry is stored in TranspositionTable
// Takes ply-dependent score and converts it to 'absolute' score
#[inline(always)]
pub fn adjust_score_from_ply(score: i32, ply: usize) -> i32 {
    if score == i32::MIN {
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

#[inline(always)]
pub fn has_non_pawn_material(board: &Board) -> bool {
    let side = board.stm;
    let side_pieces = board.positions.get_side_bb(side);
    let pawns = board.positions.get_piece_bb(side, Piece::Pawn);
    let king = board.positions.get_piece_bb(side, Piece::King);
    (*side_pieces & !(*pawns | *king)).any()
}
