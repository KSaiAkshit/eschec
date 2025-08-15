use crate::{consts::MAX_HASH, moves::move_info::Move};

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

#[derive(Debug, Default)]
pub struct TranspositionTable {
    entries: Vec<TranspositionEntry>,
    size: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let num_entries = (size_mb * 1024 * 1024) / TranspositionEntry::ENTRY_SIZE;
        let size = num_entries.next_power_of_two();
        Self {
            entries: vec![TranspositionEntry::default(); size],
            size,
        }
    }

    pub fn change_size(&mut self, new_size_mb: usize) -> miette::Result<()> {
        miette::ensure!(
            new_size_mb <= MAX_HASH,
            "Hash table size ({new_size_mb} MB) exceeds max allowed {MAX_HASH} MB"
        );
        let new_entries_num = (new_size_mb * 1024 * 1024) / TranspositionEntry::ENTRY_SIZE;
        let new_size = new_entries_num.next_power_of_two();
        self.entries
            .resize_with(new_size, TranspositionEntry::default);

        Ok(())
    }

    #[inline(always)]
    fn index(&self, hash: u64) -> usize {
        hash as usize & (self.size - 1)
    }

    pub fn probe(&self, hash: u64) -> Option<&TranspositionEntry> {
        let entry = &self.entries[self.index(hash)];
        if entry.hash == hash {
            Some(entry)
        } else {
            None
        }
    }

    pub fn store(&mut self, new_entry: TranspositionEntry) {
        let index = self.index(new_entry.hash);
        let entry = &mut self.entries[index];

        // Depth-Preferred Replacement:
        // Only replace an entry if the new one is from a deeper or equal search.
        // This is to prevents shallow searches, like from NMP, from overwriting
        // information from deeper searches.
        if new_entry.depth >= entry.depth {
            *entry = new_entry;
        }
    }

    pub fn clear(&mut self) {
        for e in self.entries.iter_mut() {
            *e = TranspositionEntry::default();
        }
    }
}
