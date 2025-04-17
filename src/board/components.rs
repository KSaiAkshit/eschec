use std::{
    fmt::Display,
    ops::{BitAnd, BitAndAssign, BitOr, Not},
};

#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct BitBoard(pub u64);

impl BitAndAssign for BitBoard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0
    }
}

impl BitOr for BitBoard {
    type Output = BitBoard;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for BitBoard {
    type Output = BitBoard;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Not for BitBoard {
    type Output = BitBoard;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl BitBoard {
    pub fn set(&mut self, pos: usize) {
        self.0 |= 1 << pos;
    }

    pub fn capture(&mut self, from_index: usize) {
        self.0 &= !(1 << from_index);
    }

    pub fn print_bitboard(&self) -> String {
        const LAST_BIT: u64 = 63;
        let mut out = String::new();
        for rank in 0..8 {
            for file in (0..8).rev() {
                let mask = 1u64 << (LAST_BIT - (rank * 8) - file);
                let char = if self.0 & mask != 0 { '1' } else { '0' };
                out = out + &char.to_string() + " ";
            }
            out = out.trim().to_owned();
            out += "\n";
        }
        out
    }

    pub fn lsb(&self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0.trailing_zeros() as usize)
        }
    }

    pub fn pop_lsb(&mut self) -> Option<usize> {
        let lsb = self.lsb();
        if let Some(_bit) = lsb {
            self.0 &= self.0 - 1;
        }
        lsb
    }

    pub fn get_set_bits(&self) -> Vec<usize> {
        let mut set_bits = Vec::new();
        let mut bb = self.0;
        let mut bit_position = 0;

        while bb > 0 {
            if bb & 1 == 1 {
                set_bits.push(bit_position);
            }
            bit_position += 1;
            bb >>= 1;
        }
        set_bits
    }

    pub fn contains_square(&self, index: usize) -> bool {
        (self.0 & (1 << index)) != 0
    }
}

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub enum Side {
    #[default]
    White,
    Black,
}

impl From<Side> for bool {
    fn from(value: Side) -> Self {
        value == Side::White
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Side::White => write!(f, "White"),
            Side::Black => write!(f, "Black"),
        }
    }
}

impl Not for Side {
    type Output = Side;

    fn not(self) -> Self::Output {
        self.flip()
    }
}

impl Side {
    pub const SIDES: [Side; 2] = [Side::White, Side::Black];
    // TODO: Should this consume self?
    pub fn white() -> usize {
        Side::White.index()
    }
    pub fn black() -> usize {
        Side::Black.index()
    }
    pub fn flip(&self) -> Self {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }
    pub fn index(&self) -> usize {
        match self {
            Side::White => 0,
            Side::Black => 1,
        }
    }
}

#[derive(Default, PartialEq, Eq, Debug, PartialOrd, Clone, Copy)]
pub enum Piece {
    #[default]
    Pawn,
    Bishop,
    Knight,
    Rook,
    Queen,
    King,
}
impl Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Piece::Pawn => writeln!(f, "Pawn"),
            Piece::Bishop => writeln!(f, "Bishop"),
            Piece::Knight => writeln!(f, "Knight"),
            Piece::Rook => writeln!(f, "Rook"),
            Piece::Queen => writeln!(f, "Queen"),
            Piece::King => writeln!(f, "King"),
        }
    }
}
impl Piece {
    pub const PIECES: [Piece; 6] = [
        Piece::Pawn,
        Piece::Bishop,
        Piece::Knight,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ];
    const SIDES: [Side; 2] = [Side::White, Side::Black];
    pub fn all() -> impl Iterator<Item = (Piece, Side)> {
        Self::SIDES
            .iter()
            .flat_map(move |&side| Self::PIECES.iter().map(move |&piece| (piece, side)))
    }

    pub fn all_pieces() -> impl Iterator<Item = Piece> {
        Self::PIECES.iter().copied()
    }

    pub fn colored_pieces(_side: Side) -> impl Iterator<Item = Piece> {
        Self::PIECES.iter().copied()
    }

    pub fn icon(&self, stm: Side) -> char {
        match &self {
            Piece::Pawn => {
                if stm.into() {
                    '♟'
                } else {
                    '♙'
                }
            }
            Piece::Bishop => {
                if stm.into() {
                    '♝'
                } else {
                    '♗'
                }
            }
            Piece::Knight => {
                if stm.into() {
                    '♞'
                } else {
                    '♘'
                }
            }
            Piece::Rook => {
                if stm.into() {
                    '♜'
                } else {
                    '♖'
                }
            }
            Piece::Queen => {
                if stm.into() {
                    '♛'
                } else {
                    '♕'
                }
            }
            Piece::King => {
                if stm.into() {
                    '♚'
                } else {
                    '♔'
                }
            }
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Piece::Pawn => 0,
            Piece::Knight => 1,
            Piece::Bishop => 2,
            Piece::Rook => 3,
            Piece::Queen => 4,
            Piece::King => 5,
        }
    }
    pub fn value(&self) -> u32 {
        u32::from(*self)
    }
}

impl From<Piece> for u32 {
    fn from(value: Piece) -> Self {
        match value {
            Piece::Pawn => 100,
            Piece::Knight => 320,
            Piece::Bishop => 330,
            Piece::Rook => 500,
            Piece::Queen => 900,
            Piece::King => 20000,
        }
    }
}

/// Snapshot of current board
#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct BoardState {
    /// Boards for all peices of white and black sides
    pub all_sides: [BitBoard; 2],
    /// Boards for all peices, of both colors
    /// [Pawn, Bishop, Knight, Rook, Queen, King]
    pub all_pieces: [[BitBoard; 6]; 2],
}
impl BoardState {
    pub fn update_piece_position(&mut self, piece: &Piece, side: &Side, from: Square, to: Square) {
        let from_index = from.index();
        let to_index = to.index();

        self.all_pieces[side.index()][piece.index()].capture(from_index);
        self.all_pieces[side.index()][piece.index()].set(to_index);

        self.update_all_sides();
    }

    fn update_all_sides(&mut self) {
        self.all_sides[0] = self.all_pieces[0][0]
            | self.all_pieces[0][1]
            | self.all_pieces[0][2]
            | self.all_pieces[0][3]
            | self.all_pieces[0][4]
            | self.all_pieces[0][5];

        self.all_sides[1] = self.all_pieces[1][0]
            | self.all_pieces[1][1]
            | self.all_pieces[1][2]
            | self.all_pieces[1][3]
            | self.all_pieces[1][4]
            | self.all_pieces[1][5];
    }
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

    pub const KING_SIDE: Self = Self(Self::BLACK_00 | Self::WHITE_00);
    pub const QUEEN_SIDE: Self = Self(Self::BLACK_000 | Self::WHITE_000);
    pub const WHITE_CASTLING: Self = Self(Self::WHITE_00 | Self::WHITE_000);
    pub const BLACK_CASTLING: Self = Self(Self::BLACK_00 | Self::BLACK_000);
    pub const ANY_CASTLING: Self = Self(Self::BLACK_CASTLING.0 | Self::WHITE_CASTLING.0);
    pub fn add_right(&mut self, rights: CastlingRights) {
        self.0 |= rights.0;
    }
    pub fn all() -> Self {
        Self::ANY_CASTLING
    }
    pub fn allows(&self, rights: CastlingRights) -> bool {
        self.0 & rights.0 != Self::NO_CASTLING
    }
    pub fn black_only() -> Self {
        Self::BLACK_CASTLING
    }
    pub fn empty() -> Self {
        Self(Self::NO_CASTLING)
    }
    pub fn is_empty(&self) -> bool {
        self.0 == 0b0000
    }
    pub fn king_side() -> Self {
        Self::KING_SIDE
    }
    pub fn queen_side() -> Self {
        Self::QUEEN_SIDE
    }
    pub fn remove_right(&mut self, rights: CastlingRights) {
        self.0 &= rights.0
    }
    pub fn white_only() -> Self {
        Self::WHITE_CASTLING
    }
}

impl BitOr<CastlingRights> for CastlingRights {
    type Output = CastlingRights;

    fn bitor(self, rhs: CastlingRights) -> Self::Output {
        Self(self.0 | rhs.0)
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
        Self::empty()
    }
}

/// Represents a single square on the board.
/// # Representation
/// 1 is A1 \
/// 2 is B1 \
/// 64 is H8
/// ```text
///    None,
/// ranks -------------------------------->
/// files
///  | v(bit 56)
///  | A8, B8, C8, D8, E8, F8, G8, H8,  <- h1 (bit 63) // 7
///  | A7, B7, C7, D7, E7, F7, G7, H7,// 6
///  | A6, B6, C6, D6, E6, F6, G6, H6,// 5
///  | A5, B5, C5, D5, E5, F5, G5, H5,// 4
///  | A4, B4, C4, D4, E4, F4, G4, H4,// 3
///  | A3, B3, C3, D3, E3, F3, G3, H3,// 2
///  | A2, B2, C2, D2, E2, F2, G2, H2,// 1
///  v A1, B1, C1, D1, E1, F1, G1, H1,  <- h1 (bit 7) // 0
///    ^(bit 0)
///```
#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Square(usize);
impl Square {
    /// Returns a Square from a given index. Will return None if index is out of bounds
    /// index should be [0, 63]
    pub fn new(index: usize) -> Option<Self> {
        if index < 64 {
            return Some(Self(index));
        }
        None
    }

    /// Returns a Square from a given File and Rank.
    /// Will return None if either File or Rank are out of bounds.
    /// Rank < 7, File < 8
    pub fn from_coords(file: usize, rank: usize) -> Option<Self> {
        if file < 7 && rank < 8 {
            return Some(Square(rank * 8 + file));
        }
        None
    }
    pub fn enpassant_from_index(file: char, rank: char) -> miette::Result<Self> {
        let file = file.to_ascii_lowercase();
        if !('a'..='g').contains(&file) {
            return Err(
                miette::Error::msg("given file isn't valid. Valid file = ['a'..='g']")
                    .context(format!("input file: {}", file)),
            );
        }
        if rank != '3' && rank != '6' {
            return Err(
                miette::Error::msg("given rank isn't valid. Valid rank = '3' or '6'")
                    .context(format!("input rank: {}", rank)),
            );
        }
        let col_index = file as usize - 'a' as usize;
        let row_index = if rank == '3' { 2 } else { 5 };
        let square_index = row_index * 8 + col_index;
        Ok(Square(square_index))
    }
    pub fn coords(&self) -> (usize, usize) {
        let file = self.0 / 8;
        let rank = self.0 % 8;
        (file, rank)
    }

    /// NOTE: Rank is 1 indexed
    pub fn rank(&self) -> usize {
        self.0 % 8 + 1
    }

    /// NOTE: File is 1 indexed
    pub fn file(&self) -> usize {
        self.0 / 8 + 1
    }

    pub fn index(&self) -> usize {
        self.0
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
    use crate::board::Board;

    use super::*;

    #[test]
    fn test_print_bitboard() {
        let out = "0 0 0 0 0 0 0 0
0 0 0 0 0 0 0 0
0 0 0 0 0 0 0 0
0 0 0 0 0 0 0 0
0 0 0 0 1 0 0 0
0 0 0 0 0 0 0 0
0 0 0 0 0 0 0 0
0 0 0 0 0 0 0 0
";
        let num = 268_435_456;
        let b = BitBoard(num);
        assert_eq!(out, b.print_bitboard())
    }

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

    #[test]
    fn test_en_passent_from_square() {
        assert!(Square::enpassant_from_index('A', '2').is_err());
        assert!(Square::enpassant_from_index('A', '3').is_ok());
        assert!(Square::enpassant_from_index('B', '3').is_ok());
        assert!(Square::enpassant_from_index('B', '6').is_ok());
    }

    #[test]
    fn test_make_move() {
        let out = "0 0 0 0 0 0 0 0
0 0 0 0 0 0 0 0
0 0 0 0 0 0 0 0
0 0 0 0 0 0 0 0
0 0 0 0 0 0 0 0
1 0 0 0 0 0 0 0
0 1 1 1 1 1 1 1
1 1 1 1 1 1 1 1
";
        let mut board = Board::new();
        assert!(board
            .make_move(Square::new(8).unwrap(), Square::new(16).unwrap())
            .is_ok());
        let o = board.positions.all_sides[0].print_bitboard();
        assert_eq!(out, o);
    }
}
