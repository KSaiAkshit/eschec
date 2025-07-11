use crate::moves::move_info::Move;

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

pub struct TranspositionTable {
    entries: Vec<TranspositionEntry>,
    size: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TranspositionEntry>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;
        let size = num_entries.next_power_of_two();
        Self {
            entries: vec![TranspositionEntry::default(); size],
            size,
        }
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

        // Always replace scheme.
        // TODO: Maybe try depth-preferred replacement
        *entry = new_entry;
    }
}
