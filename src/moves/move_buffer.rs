use std::slice::SliceIndex;

use crate::{consts::MAX_MOVES, prelude::Move};

#[derive(Clone, Copy, Debug)]
pub struct MoveBuffer {
    moves: [Move; MAX_MOVES],
    len: usize,
}

impl Default for MoveBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveBuffer {
    pub const fn new() -> Self {
        Self {
            moves: [Move(0); MAX_MOVES],
            len: 0,
        }
    }
    pub fn push(&mut self, m: Move) {
        assert!(self.len < MAX_MOVES, "MoveBuffer Overflow!");
        self.moves[self.len] = m;
        self.len += 1;
    }

    /// Returns true if the buffer contains the given move.
    #[inline(always)]
    pub fn contains(&self, m: &Move) -> bool {
        self.moves[..self.len].contains(m)
    }

    /// Returns true if the buffer is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of moves in the buffer.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Clears the buffer (sets len to zero).
    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns a slice of the moves.
    #[inline(always)]
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }

    /// Returns  the first move, or None if the buffer is empty.
    pub fn first(&self) -> Option<Move> {
        if self.len > 0 {
            Some(self.moves[0])
        } else {
            None
        }
    }

    /// Returns the last move, or None if empty.
    pub fn last(&self) -> Option<Move> {
        if self.len > 0 {
            Some(self.moves[self.len - 1])
        } else {
            None
        }
    }

    pub fn retain<F>(&mut self, mut pred: F)
    where
        F: FnMut(&Move) -> bool,
    {
        let mut write = 0;
        for read in 0..self.len {
            if pred(&self.moves[read]) {
                if write != read {
                    self.moves[write] = self.moves[read];
                }
                write += 1;
            }
        }
        self.len = write
    }

    pub fn get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<Self>,
    {
        index.get(self)
    }

    /// Returns a mutable slice of the moves.
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [Move] {
        &mut self.moves[..self.len]
    }

    /// Returns an iterator over the moves.
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &Move> {
        self.moves[..self.len].iter()
    }

    /// Returns a mutable iterator over the moves.
    #[inline(always)]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Move> {
        self.moves[..self.len].iter_mut()
    }
}

pub struct MoveBufferIter<'a> {
    buf: &'a MoveBuffer,
    pos: usize,
}

impl<'a> Iterator for MoveBufferIter<'a> {
    type Item = &'a Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.buf.len {
            let item = &self.buf.moves[self.pos];
            self.pos += 1;
            Some(item)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.buf.len - self.pos;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for MoveBufferIter<'a> {
    fn len(&self) -> usize {
        self.buf.len - self.pos
    }
}

impl<'a> IntoIterator for &'a MoveBuffer {
    type Item = &'a Move;
    type IntoIter = MoveBufferIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        MoveBufferIter { buf: self, pos: 0 }
    }
}

impl<'a> IntoIterator for &'a mut MoveBuffer {
    type Item = &'a mut Move;
    type IntoIter = std::slice::IterMut<'a, Move>;
    fn into_iter(self) -> Self::IntoIter {
        self.moves[..self.len].iter_mut()
    }
}

pub struct MoveBufferIntoIter {
    buf: MoveBuffer,
    pos: usize,
}

impl Iterator for MoveBufferIntoIter {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.buf.len {
            let item = self.buf.moves[self.pos];
            self.pos += 1;
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.buf.len - self.pos;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for MoveBufferIntoIter {
    fn len(&self) -> usize {
        self.buf.len - self.pos
    }
}

impl IntoIterator for MoveBuffer {
    type Item = Move;
    type IntoIter = MoveBufferIntoIter;
    fn into_iter(self) -> Self::IntoIter {
        MoveBufferIntoIter { buf: self, pos: 0 }
    }
}
