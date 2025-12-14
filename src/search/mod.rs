//! Chess search implementations
//!
//! This module provides various search algorithms
//! - MiniMax + Alpha-Beta Pruning with Iterative Deepening (default)
//! - Monte Carlo Tree Search (MCTS) [TODO]
//! - Parallel search variants (Lazy SMP) [TODO]

pub mod alpha_beta;
pub mod common;
pub mod move_ordering;
pub mod move_picker;
pub mod tt;

use std::sync::{Arc, atomic::AtomicBool};

pub use common::{SearchResult, SearchStats};

use crate::{
    prelude::*,
    search::common::{SearchConfig, SearchLimits},
    tuning::params::TunableParams,
};

/// Trait that all search implementations must implement
pub trait SearchEngine: Send {
    type Output;
    /// Search for the best move from the current position
    fn search(&mut self, board: &Board) -> SearchResult;

    fn init(self, search_running: Arc<AtomicBool>) -> Self::Output;

    /// Set maximum search depth
    fn set_depth(&mut self, depth: u16);

    /// Set maximum search time
    fn set_time(&mut self, time_ms: u64);

    /// Set nodes limit
    fn set_nodes(&mut self, nodes: u64);

    /// Get current config
    fn get_config(&self) -> SearchConfig;

    /// Get current config
    fn get_limits(&self) -> SearchLimits;

    /// Get current eval params
    fn get_params(&self) -> TunableParams;

    /// Clear internal state (TT, History, etc.)
    fn clear(&mut self);

    /// Stop the current search
    fn stop(&mut self);

    /// Get search Statistics
    fn get_stats(&mut self) -> SearchStats;

    /// Clone the engine (for parallel search)
    fn clone_engine(&self) -> Box<dyn SearchEngine<Output = Self::Output>>;
}

#[cfg(test)]
mod tests {
    use crate::{search::common::SearchLimits, utils::log::init};

    use super::*;

    #[test]
    fn test_null_move_pruning() {
        init();
        // let _ = utils::log::toggle_file_logging(true);
        let lim = SearchLimits::depth(10);
        let mut search_with_null = AlphaBetaSearch::new().with_limits(lim);
        let mut search_without_null = AlphaBetaSearch::new().with_limits(lim);

        assert!(search_without_null.get_config().enable_nmp);
        let conf = SearchConfig {
            enable_nmp: false,
            emit_info: false,
            ..Default::default()
        };
        search_without_null = search_without_null
            .with_config(conf)
            .expect("Should be able to set conf");
        assert!(!search_without_null.get_config().enable_nmp);

        let board = Board::from_fen(KIWIPETE);
        println!("{board}");

        info!("Starting with null move pruning");
        let start = std::time::Instant::now();
        let result_with = search_with_null.find_best_move(&board);
        let time_with = start.elapsed();

        info!("Starting without null move pruning");
        let start = std::time::Instant::now();
        let result_without = search_without_null.find_best_move(&board);
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
