use std::{fmt::Display, ops::BitOr};

#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct BitBoard(pub u64);

impl BitOr for BitBoard {
    type Output = BitBoard;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitBoard {
    pub fn set(&mut self, position: usize) {
        // dbg!(position);
        let mask = 1 << position;
        self.0 ^= mask;
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub enum Side {
    White,
    Black,
}

impl Default for Side {
    fn default() -> Self {
        Self::White
    }
}
impl Side {
    pub fn flip(&self) -> Self {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }
}

pub struct Pieces;
impl Pieces {
    pub const PAWN: usize = 0;
    pub const BISHOP: usize = 1;
    pub const KNIGHT: usize = 2;
    pub const ROOK: usize = 3;
    pub const QUEEN: usize = 4;
    pub const KING: usize = 5;
}

#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Position {
    /// Boards for all peices of white and black sides
    pub all_sides: [BitBoard; 2],
    /// Boards for all peices, of both colors
    pub all_pieces: [[BitBoard; 6]; 2],
}

/// Castling rights are stored in a [`u8`], which is divided into the following parts:
/// ```text
/// 0 1 0 1   1                1               0                0
/// ^^^^^^^   ^                ^               ^                ^
/// unused    Black queen side Black king side White queen side White king side
/// ```
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct CastlingRights(pub u8);

impl CastlingRights {
    pub const NO_CASTLING: u8 = 0;
    pub const WHITE_00: u8 = 0b00000001;
    pub const WHITE_000: u8 = 0b00000010;
    pub const BLACK_00: u8 = 0b00000100;
    pub const BLACK_000: u8 = 0b00001000;

    pub const KING_SIDE: u8 = Self::BLACK_00 | Self::WHITE_00;
    pub const QUEEN_SIDE: u8 = Self::BLACK_000 | Self::WHITE_000;
    pub const WHITE_CASTLING: u8 = Self::WHITE_00 | Self::WHITE_000;
    pub const BLACK_CASTLING: u8 = Self::BLACK_00 | Self::BLACK_000;
    pub const ANY_CASTLING: u8 = Self::BLACK_CASTLING | Self::WHITE_CASTLING;
    pub fn add_right(&mut self, rights: CastlingRights) {
        self.0 |= rights.0;
    }
    pub fn all() -> Self {
        Self(Self::ANY_CASTLING)
    }
    pub fn allows(&self, rights: CastlingRights) -> bool {
        self.0 & rights.0 != Self::NO_CASTLING
    }
    pub fn black_only() -> Self {
        Self(Self::BLACK_CASTLING)
    }
    pub fn empty() -> Self {
        Self(Self::NO_CASTLING)
    }
    pub fn is_empty(&self) -> bool {
        self.0 == 0b0000
    }
    pub fn king_side() -> Self {
        Self(Self::KING_SIDE)
    }
    pub fn queen_side() -> Self {
        Self(Self::QUEEN_SIDE)
    }
    pub fn remove_right(&mut self, rights: CastlingRights) {
        self.0 &= rights.0
    }
    pub fn white_only() -> Self {
        Self(Self::WHITE_CASTLING)
    }
}

impl Display for CastlingRights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.allows(Self(Self::WHITE_00)) {
            write!(f, "K")?;
        }
        if self.allows(Self(Self::WHITE_000)) {
            write!(f, "Q")?;
        }
        if self.allows(Self(Self::BLACK_00)) {
            write!(f, "k")?;
        }
        if self.allows(Self(Self::BLACK_000)) {
            write!(f, "q")?;
        }
        if self.is_empty() {
            write!(f, "-")?;
        }
        Ok(())
    }
}
impl Default for CastlingRights {
    fn default() -> Self {
        Self(Self::ANY_CASTLING)
    }
}

/*
    None,
    A8, B8, C8, D8, E8, F8, G8, H8,// 7
    A7, B7, C7, D7, E7, F7, G7, H7,// 6
    A6, B6, C6, D6, E6, F6, G6, H6,// 5
    A5, B5, C5, D5, E5, F5, G5, H5,// 4
    A4, B4, C4, D4, E4, F4, G4, H4,// 3
    A3, B3, C3, D3, E3, F3, G3, H3,// 2
    A2, B2, C2, D2, E2, F2, G2, H2,// 1
    A1, B1, C1, D1, E1, F1, G1, H1,// 0
*/

/// Represents a single square on the board.
/// # Representation
/// 1 is A1
/// 2 is B1
/// 64 is H8
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Square(pub usize);
impl Square {
    pub fn enpassant_from_index(file: char, rank: char) -> anyhow::Result<Self> {
        if !('a'..='g').contains(&file) {
            return Err(
                anyhow::Error::msg("given file isn't valid. Valid file = ['a'..='g']")
                    .context(format!("input file: {}", file)),
            );
        }
        if rank != '3' && rank != '6' {
            return Err(
                anyhow::Error::msg("given rank isn't valid. Valid rank = '3' or '6'")
                    .context(format!("input rank: {}", rank)),
            );
        }
        let col_index = file as usize - 'a' as usize;
        let row_index = if rank == '3' { 2 } else { 5 };
        let square_index = row_index * 8 + col_index;
        Ok(Square(square_index))
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let file = (self.0 % 8) as u8 + b'A';
        let rank = 1 + (self.0 / 8) as u8 + b'0';
        write!(f, "{}{}", file as char, rank as char)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_square() {
        // Test cases for various squares
        assert_eq!(format!("{}", Square(0)), "A1");
        assert_eq!(format!("{}", Square(7)), "H1");
        assert_eq!(format!("{}", Square(8)), "A2");
        assert_eq!(format!("{}", Square(8)), "A2");
        assert_eq!(format!("{}", Square(11)), "D2");
        assert_eq!(format!("{}", Square(63)), "H8");
        assert_eq!(format!("{}", Square(18)), "C3");
        assert_eq!(format!("{}", Square(26)), "C4");
        assert_eq!(format!("{}", Square(56)), "A8");
        assert_eq!(format!("{}", Square(28)), "E4");
        assert_eq!(format!("{}", Square(63)), "H8");
    }
}
