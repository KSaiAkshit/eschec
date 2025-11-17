use crate::prelude::*;
use std::time::Duration;

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
    pub tt_hits: u64,    // TT probes that found an entry
    pub tt_cutoffs: u64, // Times TT caused a cutoff (LowerBound / UpperBound)

    // Pruning techniques stats
    pub null_move_attempts: u64, // Times null move pruning was tried
    pub null_move_cutoffs: u64,  // Times null move pruning succeeded
    pub lmr_attempts: u64,       // Times LMR was attempted
    pub lmr_research: u64,       // Times LMR failed high and re-search was needed

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
        }
    }
}

impl SearchStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calculate_nps(&mut self) {
        let time_ms = self.time_elapsed.as_millis().max(1) as u64;
        self.nps = (self.nodes_searched * 1000) / time_ms;
    }

    pub fn beta_cutoff_rate_main(&self) -> f64 {
        if self.main_search_nodes == 0 {
            0.0
        } else {
            100.0 * self.beta_cutoffs_main as f64 / self.main_search_nodes as f64
        }
    }

    pub fn beta_cutoff_rate_qs(&self) -> f64 {
        if self.qsearch_nodes == 0 {
            0.0
        } else {
            100.0 * self.beta_cutoffs_qs as f64 / self.qsearch_nodes as f64
        }
    }

    pub fn tt_hit_rate(&self) -> f64 {
        if self.main_search_nodes == 0 {
            0.0
        } else {
            100.0 * self.tt_hits as f64 / self.main_search_nodes as f64
        }
    }

    pub fn tt_cutoff_rate(&self) -> f64 {
        if self.tt_hits == 0 {
            0.0
        } else {
            100.0 * self.tt_cutoffs as f64 / self.tt_hits as f64
        }
    }

    pub fn nmp_success_rate(&self) -> f64 {
        if self.null_move_attempts == 0 {
            0.0
        } else {
            100.0 * self.null_move_cutoffs as f64 / self.null_move_attempts as f64
        }
    }

    pub fn lmr_research_rate(&self) -> f64 {
        if self.lmr_attempts == 0 {
            0.0
        } else {
            100.0 * self.lmr_research as f64 / self.lmr_attempts as f64
        }
    }

    pub fn prune_rate(&self) -> f64 {
        if self.nodes_searched == 0 {
            0.0
        } else {
            100.0 * self.pruned_nodes as f64 / self.nodes_searched as f64
        }
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
        debug!(
            target: "search_stats",
            "=> SEARCH STATISTICS (depth {})",
            self.depth_reached
        );

        debug!(
            target: "search_stats",
            "NODES total={} time={:?} nps={}",
            self.nodes_searched, self.time_elapsed, self.nps
        );

        let main_full =
            self.beta_cutoffs_main + self.exact_scores + self.fail_lows + self.mate_returns;
        let main_early = self.tt_exact_returns + self.draw_returns + self.null_move_cutoffs;
        let qs_full = self.standpat_returns + self.beta_cutoffs_qs;
        let qs_no_moves = self.qsearch_nodes - qs_full; // Nodes that searched moves but didn't cutoff

        let accounted = main_full + main_early + qs_full + qs_no_moves;
        let unaccounted = self.nodes_searched.saturating_sub(accounted);

        debug!(
            target: "search_stats",
            "NODE_TYPES main_full={} ({:.2}%) main_early={} ({:.2}%) qs={} ({:.2}%) unaccounted={} ({:.2}%)",
            main_full,
            100.0 * main_full as f64 / self.nodes_searched as f64,
            main_early,
            100.0 * main_early as f64 / self.nodes_searched as f64,
            self.qsearch_nodes,
            100.0 * self.qsearch_nodes as f64 / self.nodes_searched as f64,
            unaccounted,
            100.0 * unaccounted as f64 / self.nodes_searched as f64
        );

        debug!(
            target: "search_stats",
            "MAIN_SEARCH full_searches={} beta_cutoffs={} ({:.2}%) exact={} fail_lows={} mates={}",
            main_full, self.beta_cutoffs_main,
            100.0 * self.beta_cutoffs_main as f64 / main_full.max(1) as f64,
            self.exact_scores, self.fail_lows, self.mate_returns
        );

        debug!(
            target: "search_stats",
            "MAIN_EARLY tt_exact={} draws={} nmp={}",
            self.tt_exact_returns, self.draw_returns, self.null_move_cutoffs
        );

        debug!(
            target: "search_stats",
            "QSEARCH total={} standpat={} ({:.2}%) beta_cutoffs={} ({:.2}%) no_cutoff={} ({:.2}%)",
            self.qsearch_nodes,
            self.standpat_returns,
            100.0 * self.standpat_returns as f64 / self.qsearch_nodes.max(1) as f64,
            self.beta_cutoffs_qs,
            100.0 * self.beta_cutoffs_qs as f64 / self.qsearch_nodes.max(1) as f64,
            qs_no_moves,
            100.0 * qs_no_moves as f64 / self.qsearch_nodes.max(1) as f64
        );

        debug!(
            target: "search_stats",
            "TT hits={} ({:.2}% of total) cutoffs={} ({:.2}% of hits) hash_full={}/1000",
            self.tt_hits,
            100.0 * self.tt_hits as f64 / self.nodes_searched as f64,
            self.tt_cutoffs,
            self.tt_cutoff_rate(),
            self.hash_full
        );

        debug!(
            target: "search_stats",
            "NULL_MOVE attempts={} cutoffs={} success_rate={:.2}%",
            self.null_move_attempts, self.null_move_cutoffs, self.nmp_success_rate()
        );

        debug!(
            target: "search_stats",
            "LMR attempts={} re_searches={} re_search_rate={:.2}%",
            self.lmr_attempts, self.lmr_research, self.lmr_research_rate()
        );

        if self.asp_research > 0 {
            debug!(
                target: "search_stats",
                "ASPIRATION fail_highs={} fail_lows={} total_re_searches={}",
                self.asp_fail_high, self.asp_fail_low, self.asp_research
            );
        }

        debug!(
            target: "search_stats",
            "PRUNING total_pruned={} prune_rate={:.2}%",
            self.pruned_nodes, self.prune_rate()
        );

        self.log_cutoff_stats();
    }

    pub fn log_cutoff_stats(&self) {
        let total_cutoffs: u64 = self.cutoff_at_move.iter().sum();
        if total_cutoffs == 0 {
            return;
        }

        let avg_cutoff_index = self.avg_cutoff_index();

        debug!(
            target: "cutoff_stats",
            "CUTOFF_STATS depth={} beta_cutoffs={} avg_cutoff_at={:.2}",
            self.depth_reached, total_cutoffs, avg_cutoff_index
        );

        let histogram: Vec<String> = self
            .cutoff_at_move
            .iter()
            .take(20)
            .enumerate()
            .filter(|&(_, &count)| count > 0)
            .map(|(i, count)| format!("{}:{}", i, count))
            .collect();

        if !histogram.is_empty() {
            debug!(
                target: "cutoff_stats",
                "CUTOFF_HISTOGRAM depth={} data=[{}]",
                self.depth_reached,
                histogram.join(",")
            );
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
