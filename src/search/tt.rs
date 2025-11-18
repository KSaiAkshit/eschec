#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};

use crate::{consts::MAX_HASH, moves::move_info::Move};

const NUM_ENTRIES_PER_CLUSTER: usize = 4;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScoreTypes {
    /// Score is the exact evaluation [alpha <= score <= beta]
    Exact = 0,
    /// Score is at least this value, i.e, beta cutoff [score >= beta]
    LowerBound = 1,
    /// Score is at most this value, i.e, alpha not improved [score <= alpha]
    UpperBound = 2,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TranspositionEntry {
    // 4 bytes for the key. Storing only the lower 32 bits of the hash
    // Collisions are handled by checking the full hash on the board
    key: u32,
    // 2 byts for the best move found
    best_move: Move,
    // 2 bytes for the eval score
    score: i16,
    // 1 byte for the depth
    depth: u8,
    // 1 byte for flags
    flags: u8,
}

impl TranspositionEntry {
    pub const ENTRY_SIZE: usize = std::mem::size_of::<TranspositionEntry>();

    const SCORE_TYPE_MASK: u8 = 0b11; // Use lower 2 bits for score type
    const AGE_SHIFT: u8 = 2; // Shift by 2 to get the age bits

    pub fn new(
        hash: u64,
        best_move: Move,
        score: i32,
        depth: u8,
        score_type: ScoreTypes,
        age: u8,
    ) -> Self {
        let packed_score = score as i16;

        let packed_flags = (age << Self::AGE_SHIFT) | (score_type as u8);

        Self {
            key: hash as u32,
            best_move,
            score: packed_score,
            depth,
            flags: packed_flags,
        }
    }

    /// Checks if the key of this entry matches the lower 32 bits of full hash
    #[inline]
    pub fn matches(&self, hash: u64) -> bool {
        self.key == (hash as u32)
    }

    /// Unpacks and returns the score type
    #[inline]
    pub fn get_score_type(&self) -> ScoreTypes {
        // SAFETY: this is safe because ScoreType is repr(u8) and only valid values are stored
        unsafe { std::mem::transmute(self.flags & Self::SCORE_TYPE_MASK) }
    }

    #[inline]
    /// Return evaluation score
    pub fn get_score(&self) -> i32 {
        self.score as i32
    }

    /// Unpacks and returns the age of the entry
    #[inline]
    pub fn get_age(&self) -> u8 {
        self.flags >> Self::AGE_SHIFT
    }

    // Simple getters for the remaining fields
    #[inline]
    pub fn get_best_move(&self) -> Move {
        self.best_move
    }
    #[inline]
    pub fn get_depth(&self) -> u8 {
        self.depth
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
            if entry.matches(hash) {
                return Some(entry);
            }
        }
        None
    }

    /// Store an entry with a Depth+Age prefered replacement strategy
    pub fn store(&mut self, new_entry: TranspositionEntry) {
        let index = self.index(new_entry.key as u64);
        let cluster = &mut self.clusters[index];

        // Check if an entry for the same position already exists.
        // Only replace an entry if the new one is from a deeper or equal search.
        // This is to prevents shallow searches, like from NMP, from overwriting
        // information from deeper searches.
        for entry in &mut cluster.entries {
            if entry.key == new_entry.key {
                if new_entry.depth >= entry.depth {
                    *entry = new_entry;
                }
                return;
            }
        }

        // Depth + Age Preferred Replacement:
        // if no existing entry was found, replace one of the current 'worst' entries
        let mut replace_idx = 0;
        let mut worst_score = i32::MAX;

        for (i, entry) in cluster.entries.iter().enumerate() {
            // Score an entry based on its age and depth.
            // A lower score is worse and a better candidate for replacement.
            // - Prioritize replacing entries from older search cycles.
            // - Among entries from the same cycle, replace the one with the shallowest depth.
            let age_difference = new_entry.get_age().wrapping_sub(entry.get_age());
            let score = (entry.depth as i32) - (age_difference as i32) * 4; // Weight age more heavily

            if score < worst_score {
                worst_score = score;
                replace_idx = i;
            }
        }

        // Replace the entry tat is the worst
        cluster.entries[replace_idx] = new_entry;
    }

    pub fn clear(&mut self) {
        for c in self.clusters.iter_mut() {
            *c = Cluster::default();
        }
    }

    /// Return the hash table fullness in per-mille (0-1000)
    pub fn hash_full(&self) -> u16 {
        let sample_size = 1000.min(self.size);
        let mut filled = 0;

        for i in 0..sample_size {
            let cluster = &self.clusters[i];
            for entry in &cluster.entries {
                if entry.key != 0 {
                    filled += 1;
                    break; // Only count cluster as filled if atleast one entry is used
                }
            }
        }

        ((filled * 1000) / sample_size) as u16
    }
}
