use std::mem::MaybeUninit;
use std::slice::{Iter, IterMut, SliceIndex};

#[cfg(feature = "parallel")]
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator};

use crate::{consts::MAX_MOVES, prelude::Move};

/// A fixed-size, stack-allocated buffer for storing chess moves.
///
/// It holds up to `MAX_MOVES` (256) and tracks the
/// number of moves currently stored.
#[derive(Debug)]
pub struct MoveBuffer {
    moves: [MaybeUninit<Move>; MAX_MOVES],
    len: usize,
}

impl Clone for MoveBuffer {
    fn clone(&self) -> Self {
        let mut new_buf = Self::new();
        for i in 0..self.len {
            unsafe { new_buf.push(self.get_unchecked(i)) }
        }
        new_buf
    }
}

impl Default for MoveBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveBuffer {
    /// Creates a new, empty `MoveBuffer`.
    pub const fn new() -> Self {
        Self {
            // SAFETY: MaybeUninit::uninit() is valid for MaybeUninit types.
            // This is zero-cost and safe.
            moves: [MaybeUninit::uninit(); MAX_MOVES],
            len: 0,
        }
    }

    /// Adds a move to the end of the buffer.
    ///
    /// # Panics
    /// Panics if the buffer is full (debug mode).
    #[inline(always)]
    pub fn push(&mut self, m: Move) {
        debug_assert!(self.len < MAX_MOVES, "MoveBuffer overflow!");
        // SAFETY: Assuming `MAX_MOVES` is enough for chess
        // Using get_unchecked for hot paths
        unsafe {
            *self.moves.get_unchecked_mut(self.len).as_mut_ptr() = m;
        }
        self.len += 1;
    }

    /// Returns true if the buffer contains the given move.
    #[inline(always)]
    pub fn contains(&self, m: &Move) -> bool {
        self.as_slice().contains(m)
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

    /// Clears the buffer (only sets len to zero).
    #[inline(always)]
    pub const fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns a slice containing all the moves in the buffer.
    #[inline(always)]
    pub fn as_slice(&self) -> &[Move] {
        // SAFETY: We only expose the slice up to `len`, which is initialized
        // Casting *const MaybeUninit<Move> to *const Move is valid because they have the same layout.
        unsafe { std::slice::from_raw_parts(self.moves.as_ptr() as *const Move, self.len()) }
    }

    /// Returns a mutable slice containing all the moves in the buffer.
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [Move] {
        // SAFETY: We only expose the slice up to `len`, which is initialized
        // Cast *const MaybeUnint<Move> to * const Move
        unsafe { std::slice::from_raw_parts_mut(self.moves.as_mut_ptr() as *mut Move, self.len()) }
    }

    /// Returns a reference to the first move, or `None` if the buffer is empty.
    pub fn first(&self) -> Option<&Move> {
        self.as_slice().first()
    }

    /// Returns a reference to the last move, or `None` if the buffer is empty.
    pub fn last(&self) -> Option<&Move> {
        self.as_slice().last()
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In-place filtering of the buffer. Moves for which `pred` returns `false` are removed.
    pub fn retain<F>(&mut self, mut pred: F)
    where
        F: FnMut(&Move) -> bool,
    {
        let mut len = 0;
        for i in 0..self.len {
            // SAFETY: `i` is within bounds of initialized items
            unsafe {
                if pred(&self.get_unchecked(i)) {
                    // OPTIM: maybe use memcpy here
                    self.moves.swap(len, i);
                    len += 1;
                }
            }
        }
        self.len = len;
    }

    /// Helper for internal use.
    /// Caller should guarentee that 0 <= index <= len
    unsafe fn get_unchecked(&self, index: usize) -> Move {
        unsafe { self.moves.get_unchecked(index).assume_init_read() }
    }

    /// Returns a reference to a move or sub-slice, or `None` if out of bounds.
    pub fn get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[Move]>,
    {
        index.get(self.as_slice())
    }

    /// Returns an iterator over the moves.
    /// This is a convenience wrapper around the slice's iterator.
    #[inline(always)]
    pub fn iter(&self) -> Iter<'_, Move> {
        self.as_slice().iter()
    }

    /// Returns a mutable iterator over the moves.
    /// This is a convenience wrapper around the slice's mutable iterator.
    #[inline(always)]
    pub fn iter_mut(&mut self) -> IterMut<'_, Move> {
        self.as_mut_slice().iter_mut()
    }
}

/// Enables iterating over a `&MoveBuffer` with a `for` loop.
///
/// This implementation allows for read-only iteration over the moves
/// contained within the buffer by delegating to the standard slice iterator.
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
    type IntoIter = Iter<'a, Move>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

/// Enables iterating mutably over a `&mut MoveBuffer` with a `for` loop.
///
/// This implementation allows for mutable iteration, enabling in-place
/// modification of the moves within the buffer. It delegates to the standard slice
/// iterator.
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
/// assert_eq!(buffer.first(), Some(&Move::new(2, 3, 0)));
/// ```
impl<'a> IntoIterator for &'a mut MoveBuffer {
    type Item = &'a mut Move;
    type IntoIter = IterMut<'a, Move>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

/// Enables consuming a `MoveBuffer` with a `for` loop, yielding moves by value.
///
/// This implementation allows for iteration that takes ownership of the
/// moves from the buffer.
///
/// **Note:** This operation involves a heap allocation to convert the internal
/// array slice into a `Vec` before iterating. It is less performant than borrowed iteration.
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
pub struct MoveBufferIntoIter {
    buf: MoveBuffer,
    pos: usize,
}

impl Iterator for MoveBufferIntoIter {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.buf.len {
            unsafe {
                let item = self.buf.get_unchecked(self.pos);
                self.pos += 1;
                Some(item)
            }
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.buf.len - self.pos;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for MoveBufferIntoIter {}

impl IntoIterator for MoveBuffer {
    type Item = Move;
    type IntoIter = MoveBufferIntoIter;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        MoveBufferIntoIter { buf: self, pos: 0 }
    }
}

/// Implementation for parallel iteration over a borrowed `MoveBuffer` (`&MoveBuffer`).
#[cfg(feature = "parallel")]
impl<'a> IntoParallelIterator for &'a MoveBuffer {
    type Item = &'a Move;
    type Iter = rayon::slice::Iter<'a, Move>;

    fn into_par_iter(self) -> Self::Iter {
        self.as_slice().par_iter()
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
        self.as_mut_slice().par_iter_mut()
    }
}

/// Implementation for parallel iteration that consumes the `MoveBuffer`.
///
/// This allows you to use `.into_par_iter()` to get a parallel iterator that
/// takes ownership of the moves.
///
/// **Note:** This operation involves a heap allocation.
#[cfg(feature = "parallel")]
impl IntoParallelIterator for MoveBuffer {
    type Item = Move;
    type Iter = rayon::vec::IntoIter<Move>;

    fn into_par_iter(self) -> Self::Iter {
        self.as_slice().to_vec().into_par_iter()
    }
}
