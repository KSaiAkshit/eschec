use std::slice::SliceIndex;

#[cfg(feature = "parallel")]
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator};

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
    pub const fn push(&mut self, m: Move) {
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
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of moves in the buffer.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Clears the buffer (sets len to zero).
    #[inline(always)]
    pub const fn clear(&mut self) {
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

/// An iterator that yields references to the moves in a `MoveBuffer`.
///
/// This struct is created by the [`iter()`](MoveBuffer::iter) method on `MoveBuffer`
/// or when iterating over a `&MoveBuffer` in a `for` loop. You should not need to
/// construct this manually.
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

/// Enables iterating over a `&MoveBuffer` with a `for` loop.
///
/// This implementation allows for idiomatic, read-only iteration over the moves
/// contained within the buffer.
///
/// # Example
/// ```
/// # use eschec::prelude::{Move, MoveBuffer};
/// let mut buffer = MoveBuffer::new();
/// buffer.push(Move::new(0, 1, 0));
///
/// for mov in &buffer {
///     // mov is a &Move
///     println!("{:?}", mov);
/// }
/// ```
impl<'a> IntoIterator for &'a MoveBuffer {
    type Item = &'a Move;
    type IntoIter = MoveBufferIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        MoveBufferIter { buf: self, pos: 0 }
    }
}

/// Enables iterating mutably over a `&mut MoveBuffer` with a `for` loop.
///
/// This implementation allows for idiomatic, mutable iteration, enabling in-place
/// modification of the moves within the buffer. It delegates to the standard slice
/// iterator for optimal performance.
///
/// # Example
/// ```
/// # use eschec::prelude::{Move, MoveBuffer};
/// let mut buffer = MoveBuffer::new();
/// buffer.push(Move::new(0, 1, 0));
///
/// for mov in &mut buffer {
///     // mov is a &mut Move, so it can be modified.
///     *mov = Move::new(2, 3, 0);
/// }
/// assert_eq!(buffer.first(), Some(Move::new(2, 3, 0)));
/// ```
impl<'a> IntoIterator for &'a mut MoveBuffer {
    type Item = &'a mut Move;
    type IntoIter = std::slice::IterMut<'a, Move>;
    fn into_iter(self) -> Self::IntoIter {
        self.moves[..self.len].iter_mut()
    }
}

/// An iterator that consumes a `MoveBuffer` and yields its moves by value.
///
/// This struct is created when iterating over a `MoveBuffer` by value in a `for` loop.
/// You should not need to construct this manually.
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

/// Enables consuming a `MoveBuffer` with a `for` loop, yielding moves by value.
///
/// This implementation allows for idiomatic iteration that takes ownership of the
/// moves from the buffer.
///
/// # Example
/// ```
/// # use eschec::prelude::{Move, MoveBuffer};
/// let mut buffer = MoveBuffer::new();
/// buffer.push(Move::new(0, 1, 0));
///
/// for mov in buffer {
///     // mov is a Move (by value), not a reference.
///     println!("{:?}", mov);
/// }
/// // `buffer` has been moved and cannot be used here anymore.
/// ```
impl IntoIterator for MoveBuffer {
    type Item = Move;
    type IntoIter = MoveBufferIntoIter;
    fn into_iter(self) -> Self::IntoIter {
        MoveBufferIntoIter { buf: self, pos: 0 }
    }
}

/// Implementation for parallel iteration over borrowed `MoveBuffer` (`&MoveBuffer`).
#[cfg(feature = "parallel")]
impl<'a> IntoParallelIterator for &'a MoveBuffer {
    type Item = &'a Move;
    type Iter = rayon::slice::Iter<'a, Move>;

    fn into_par_iter(self) -> Self::Iter {
        self.moves[..self.len].par_iter()
    }
}

/// Implementation for parallel iteration over a mutable `MoveBuffer` (`&mut MoveBuffer`).
///
/// This allows you to use `.par_iter_mut()` to modify moves in parallel.
#[cfg(feature = "parallel")]
impl<'a> IntoParallelIterator for &'a mut MoveBuffer {
    type Item = &'a mut Move;
    type Iter = rayon::slice::IterMut<'a, Move>;

    fn into_par_iter(self) -> Self::Iter {
        self.moves[..self.len].par_iter_mut()
    }
}

/// Implementation for parallel iteration that consumes the `MoveBuffer`.
///
/// This allows you to use `.into_par_iter()` to get a parallel iterator that
/// takes ownership of the moves. This is less common but can be useful.
#[cfg(feature = "parallel")]
impl IntoParallelIterator for MoveBuffer {
    type Item = Move;
    type Iter = rayon::vec::IntoIter<Move>;

    fn into_par_iter(self) -> Self::Iter {
        // To create a consuming parallel iterator, the slice first needs to be converted
        //  into an owned Vec, and then use its parallel iterator.
        let vec: Vec<Move> = self.moves[..self.len].to_vec();
        vec.into_par_iter()
    }
}
