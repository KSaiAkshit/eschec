#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};

use crate::{consts::MAX_HASH, moves::move_info::Move};

const NUM_ENTRIES_PER_CLUSTER: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScoreTypes {
    /// Score is the exact evaluation [alpha <= score <= beta]
    Exact,
    /// Score is at least this value, i.e, beta cutoff [score >= beta]
    LowerBound,
    /// Score is at most this value, i.e, alpha not improved [score <= alpha]
    UpperBound,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TranspositionEntry {
    pub hash: u64,
    pub depth: u8,
    pub score: i32,
    pub score_type: ScoreTypes,
    pub best_move: Move,
}

impl TranspositionEntry {
    pub const ENTRY_SIZE: usize = std::mem::size_of::<TranspositionEntry>();
}
impl Default for TranspositionEntry {
    fn default() -> Self {
        Self {
            hash: Default::default(),
            depth: Default::default(),
            score: Default::default(),
            score_type: ScoreTypes::Exact,
            best_move: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Cluster {
    entries: [TranspositionEntry; NUM_ENTRIES_PER_CLUSTER],
}

impl Cluster {
    pub const CLUSTER_SIZE: usize = std::mem::size_of::<Cluster>();
}

#[derive(Debug)]
pub struct TranspositionTable {
    clusters: Vec<Cluster>,
    size: usize,
}

/// Default to 16 MB Transposition Table
impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new(16)
    }
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let num_entries = (size_mb * 1024 * 1024) / TranspositionEntry::ENTRY_SIZE;
        let num_clusters = (num_entries / NUM_ENTRIES_PER_CLUSTER).next_power_of_two();
        Self {
            clusters: vec![Cluster::default(); num_clusters],
            size: num_clusters,
        }
    }

    pub fn change_size(&mut self, new_size_mb: usize) -> miette::Result<()> {
        miette::ensure!(
            new_size_mb <= MAX_HASH,
            "Hash table size ({new_size_mb} MB) exceeds max allowed {MAX_HASH} MB"
        );
        let new_entries_num = (new_size_mb * 1024 * 1024) / TranspositionEntry::ENTRY_SIZE;
        let new_size = (new_entries_num / NUM_ENTRIES_PER_CLUSTER).next_power_of_two();
        self.clusters.resize_with(new_size, Cluster::default);
        self.size = new_size;

        Ok(())
    }

    #[inline(always)]
    fn index(&self, hash: u64) -> usize {
        hash as usize & (self.size - 1)
    }

    pub fn probe(&self, hash: u64) -> Option<&TranspositionEntry> {
        let index = self.index(hash);
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let ptr = self.clusters.as_ptr().cast::<i8>();
            _mm_prefetch(ptr.add(index * Cluster::CLUSTER_SIZE), _MM_HINT_T0);
        }
        let cluster = &self.clusters[index];
        for i in 0..NUM_ENTRIES_PER_CLUSTER {
            let entry = &cluster.entries[i];
            if entry.hash == hash {
                return Some(entry);
            }
        }
        None
    }

    pub fn store(&mut self, new_entry: TranspositionEntry) {
        let index = self.index(new_entry.hash);
        let cluster = &mut self.clusters[index];

        // Depth-Preferred Replacement:
        // Only replace an entry if the new one is from a deeper or equal search.
        // This is to prevents shallow searches, like from NMP, from overwriting
        // information from deeper searches.
        for entry in &mut cluster.entries {
            if entry.hash == new_entry.hash {
                if new_entry.depth >= entry.depth {
                    *entry = new_entry;
                }
                return;
            }
        }

        let mut replace_idx = 0;
        let mut min_depth = u8::MAX;
        for (i, entry) in cluster.entries.iter().enumerate() {
            if entry.depth < min_depth {
                min_depth = entry.depth;
                replace_idx = i;
            }
        }

        if new_entry.depth >= min_depth {
            cluster.entries[replace_idx] = new_entry;
        }
    }

    pub fn clear(&mut self) {
        for c in self.clusters.iter_mut() {
            *c = Cluster::default();
        }
    }
}
