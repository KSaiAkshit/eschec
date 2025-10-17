use crate::prelude::*;
use std::time::Duration;

/// Common statistics for all search types
#[derive()]
pub struct SearchStats {
    pub nodes_searched: u64,
    pub depth_reached: u64,
    pub time_elapsed: Duration,
    pub nps: u64,
    pub tb_hits: u64,
    pub hash_full: u16, // per-mille
}

/// Cut-off stats
/// Useful to see how good move ordering is
#[derive(Debug, Clone)]
pub struct CutoffStats {
    pub total_nodes: u64,
    pub cutoff_nodes: u64,
    pub cutoff_at_move: [u64; 64],
}

impl Default for CutoffStats {
    fn default() -> Self {
        Self {
            total_nodes: Default::default(),
            cutoff_nodes: Default::default(),
            cutoff_at_move: [0; 64],
        }
    }
}

impl CutoffStats {
    pub fn log_summary(&self, depth: u8) {
        if self.total_nodes == 0 {
            return;
        }

        let cutoff_rate = 100.0 * self.cutoff_nodes as f64 / self.total_nodes as f64;

        // Calculate average move index at cutoff
        let total_cutoff_moves: u64 = self
            .cutoff_at_move
            .iter()
            .enumerate()
            .map(|(i, &count)| i as u64 * count)
            .sum();
        let avg_cutoff_index = if self.cutoff_nodes > 0 {
            total_cutoff_moves as f64 / self.cutoff_nodes as f64
        } else {
            0.0
        };

        debug!(
            target: "cutoff_stats",
            "CUTOFF_STATS depth={} total_nodes={} cutoff_nodes={} cutoff_rate={:.2} avg_cutoff_at={:.2}",
            depth, self.total_nodes, self.cutoff_nodes, cutoff_rate, avg_cutoff_index
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
                depth,
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
            collect_stats: false, // Disabled for perf
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
