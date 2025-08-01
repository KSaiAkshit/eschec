use crate::{consts::MAX_MOVES, prelude::Move};

#[derive(Clone, Debug)]
pub struct MoveBuffer {
    moves: [Move; MAX_MOVES],
    len: usize,
}

impl MoveBuffer {
    pub const fn new() -> Self {
        Self {
            moves: [Move(0); MAX_MOVES],
            len: 0,
        }
    }
    pub const fn push(&mut self, m: Move) {
        debug_assert!(self.len < MAX_MOVES, "MoveBuffer Overflow!");
        self.moves[self.len] = m;
        self.len += 1;
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
