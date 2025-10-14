use crate::{
    prelude::*,
    search::move_ordering::{MainSearchPolicy, MoveScoringPolicy, QSearchPolicy},
};

/// MovePicker: Efficiently picks moves one at a time without re-scoring.
///
/// Scores all moves once upfront, then uses O(n) selection to find the best
/// remaining move on each call to `next_best()`. This is faster than full sorting
/// when beta cutoffs happen early (which is common with good move ordering).
///
/// # Usage
/// ```ignore
/// let mut picker = MovePicker::new(
///     &board,
///     legal_moves.as_mut_slice(),
///     &killers,
///     Some(tt_move),
///     &history,
/// );
///
/// while let Some(mv) = picker.next_best() {
///     // Try move...
///     if beta_cutoff {
///         break; // Saved scoring/selecting remaining moves!
///     }
/// }
/// ```
pub struct MovePicker<'a> {
    moves: &'a mut [Move],
    scores: [i32; MAX_MOVES],
    current: usize,
}

impl<'a> MovePicker<'a> {
    /// Creates a new MovePicker.
    ///
    /// Scores all moves once using the given policy. Subsequent calls to
    /// `next_best()` only perform O(n) selection without re-scoring.
    ///
    /// # Arguments
    /// * `board` - Current board position (for scoring)
    /// * `moves` - Mutable slice of legal moves to pick from
    /// * `killers` - Killer moves for this ply
    /// * `tt_move` - Transposition table move (gets highest priority)
    /// * `history` - History heuristic table
    pub fn new(
        board: &'a Board,
        moves: &'a mut [Move],
        killers: &[Option<Move>; 2],
        tt_move: Option<Move>,
        history: &[[i32; 64]; 64],
    ) -> Self {
        debug_assert!(moves.len() <= MAX_MOVES, "Too many moves");

        let mut scores = [0i32; MAX_MOVES];

        for (i, &mv) in moves.iter().enumerate() {
            scores[i] = MainSearchPolicy::score(board, mv, killers, tt_move, history);
        }

        Self {
            moves,
            scores,
            current: 0,
        }
    }

    /// Constructor for quiescence search.
    ///
    /// # Arguments
    /// * `board` - Current board position (for scoring)
    /// * `moves` - Mutable slice of legal moves to pick from
    pub fn new_qsearch(board: &'a Board, moves: &'a mut [Move]) -> Self {
        debug_assert!(moves.len() <= MAX_MOVES, "Too many moves");

        let mut scores = [0i32; MAX_MOVES];

        for (i, &mv) in moves.iter().enumerate() {
            scores[i] = QSearchPolicy::score(board, mv, &[None; 2], None, &[[0; 64]; 64]);
        }

        Self {
            moves,
            scores,
            current: 0,
        }
    }

    /// Returns the next best move, or None if all moves have been picked.
    ///
    /// Finds the highest-scored move in O(n) time without re-scoring.
    /// Swaps it to the current position and increments the cursor.
    #[inline]
    pub fn next_best(&mut self) -> Option<Move> {
        if self.current >= self.moves.len() {
            return None;
        }

        // Find the index of the best remaining move
        let mut best_idx = self.current;
        let mut best_score = self.scores[self.current];

        for i in (self.current + 1)..self.moves.len() {
            if self.scores[i] > best_score {
                best_score = self.scores[i];
                best_idx = i;
            }
        }

        // Swap best move to current position
        self.moves.swap(self.current, best_idx);
        self.scores.swap(self.current, best_idx);

        let result = self.moves[self.current];
        self.current += 1;
        Some(result)
    }

    /// Returns the number of moves remaining to be picked.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.moves.len().saturating_sub(self.current)
    }

    /// Peeks at the best remaining move's score without picking it.
    ///
    /// Useful for futility pruning or other search heuristics.
    pub fn peek_best_score(&self) -> Option<i32> {
        if self.current >= self.scores.len() {
            return None;
        }

        let mut best_score = self.scores[self.current];
        for i in (self.current + 1)..self.moves.len() {
            if self.scores[i] > best_score {
                best_score = self.scores[i];
            }
        }

        Some(best_score)
    }

    /// Resets the picker to start from the beginning.
    ///
    /// Does NOT re-score moves. Useful if you want to iterate multiple times
    /// over the same set of moves (rare in practice).
    #[inline]
    pub fn reset(&mut self) {
        self.current = 0;
    }
}

// Implement Iterator trait for convenience
impl<'a> Iterator for MovePicker<'a> {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_best()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining();
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for MovePicker<'a> {
    fn len(&self) -> usize {
        self.remaining()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_picker_basic() {
        let board = Board::from_fen(START_FEN);
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);

        let original_count = moves.len();
        let mut picker = MovePicker::new(
            &board,
            moves.as_mut_slice(),
            &[None; 2],
            None,
            &[[0; 64]; 64],
        );

        // Should pick exactly original_count moves
        let mut picked = Vec::new();
        while let Some(mv) = picker.next_best() {
            picked.push(mv);
        }

        assert_eq!(picked.len(), original_count);

        // All picked moves should be unique
        let mut seen = std::collections::HashSet::new();
        for mv in picked {
            assert!(seen.insert(mv), "Duplicate move picked!");
        }
    }

    #[test]
    fn test_move_picker_ordering() {
        let board =
            Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);

        let mut picker = MovePicker::new(
            &board,
            moves.as_mut_slice(),
            &[None; 2],
            None,
            &[[0; 64]; 64],
        );

        // First few moves should be captures (higher scores)
        let first = picker.next_best().unwrap();
        let second = picker.next_best().unwrap();

        // At least one of the first two should be a capture
        assert!(
            first.is_capture() || second.is_capture(),
            "Expected captures to be picked first"
        );
    }

    #[test]
    fn test_move_picker_with_tt_move() {
        let board = Board::from_fen(START_FEN);
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);

        // Pick a random move as TT move
        let tt_move = moves.first().copied().unwrap();

        let mut picker = MovePicker::new(
            &board,
            moves.as_mut_slice(),
            &[None; 2],
            Some(tt_move),
            &[[0; 64]; 64],
        );

        // TT move should be picked first (highest score = TT_MOVE_SCORE)
        let first_picked = picker.next_best().unwrap();
        assert_eq!(first_picked, tt_move, "TT move should be picked first");
    }

    #[test]
    fn test_move_picker_remaining() {
        let board = Board::from_fen(START_FEN);
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);

        let total = moves.len();
        let mut picker = MovePicker::new(
            &board,
            moves.as_mut_slice(),
            &[None; 2],
            None,
            &[[0; 64]; 64],
        );

        assert_eq!(picker.remaining(), total);

        picker.next_best();
        assert_eq!(picker.remaining(), total - 1);

        picker.next_best();
        assert_eq!(picker.remaining(), total - 2);
    }

    #[test]
    fn test_move_picker_peek() {
        let board = Board::from_fen(START_FEN);
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);

        let picker = MovePicker::new(
            &board,
            moves.as_mut_slice(),
            &[None; 2],
            None,
            &[[0; 64]; 64],
        );

        let peeked_score = picker.peek_best_score();
        assert!(peeked_score.is_some());

        // Peeking shouldn't change state
        assert_eq!(picker.remaining(), moves.len());
    }

    #[test]
    fn test_move_picker_iterator() {
        let board = Board::from_fen(START_FEN);
        let mut moves = MoveBuffer::new();
        board.generate_legal_moves(&mut moves, false);

        let total = moves.len();
        let picker = MovePicker::new(
            &board,
            moves.as_mut_slice(),
            &[None; 2],
            None,
            &[[0; 64]; 64],
        );

        // Can use as iterator
        let collected: Vec<Move> = picker.collect();
        assert_eq!(collected.len(), total);
    }
}
