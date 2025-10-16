use std::{
    fmt::{Display, Write},
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not},
    str::FromStr,
};

use miette::Context;

use crate::prelude::*;

#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
#[repr(transparent)]
pub struct BitBoard(pub u64);

impl BitAndAssign for BitBoard {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0
    }
}

impl BitOrAssign for BitBoard {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl BitOr for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Not for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl BitOr for &BitBoard {
    type Output = BitBoard;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        BitBoard(self.0 | rhs.0)
    }
}

impl BitAnd for &BitBoard {
    type Output = BitBoard;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        BitBoard(self.0 & rhs.0)
    }
}

impl Not for &BitBoard {
    type Output = BitBoard;

    #[inline(always)]
    fn not(self) -> Self::Output {
        BitBoard(!self.0)
    }
}

impl BitBoard {
    #[inline(always)]
    pub const fn set(&mut self, pos: usize) {
        self.0 |= 1 << pos;
    }

    #[inline(always)]
    pub const fn capture(&mut self, index: usize) {
        self.0 &= !(1 << index);
    }

    #[inline(always)]
    pub fn pop_count(&self) -> u32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "popcnt"))]
        {
            unsafe { std::arch::x86_64::_popcnt64(self.0 as i64) as u32 }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "popcnt")))]
        {
            self.0.count_ones()
        }
    }

    pub fn print_bitboard(&self) -> String {
        const LAST_BIT: u64 = 63;
        let mut out = String::with_capacity(8 * 8 * 2);
        for rank in 0..8 {
            for file in (0..8).rev() {
                let mask = 1u64 << (LAST_BIT - (rank * 8) - file);
                let char = if self.0 & mask != 0 { '1' } else { '0' };
                write!(out, "{} ", char).expect("");
                // out = out + &char.to_string() + " ";
            }
            out = out.trim().to_owned();
            writeln!(out).unwrap();
            // out += "\n";
        }
        out
    }

    #[inline(always)]
    pub fn lsb(&self) -> Option<u64> {
        if self.0 == 0 {
            return None;
        }
        #[cfg(all(target_arch = "x86_64", target_feature = "bmi1"))]
        {
            Some(unsafe { std::arch::x86_64::_tzcnt_u64(self.0) })
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "bmi1")))]
        {
            Some(self.0.trailing_zeros() as u64)
        }
    }

    #[inline(always)]
    pub const fn const_lsb(&self) -> Option<u64> {
        if self.0 == 0 {
            return None;
        }
        Some(self.0.trailing_zeros() as u64)
    }

    #[inline(always)]
    pub fn pop_lsb(&mut self) -> u64 {
        let idx = self.0.trailing_zeros() as u64;
        #[cfg(all(target_arch = "x86_64", target_feature = "bmi1"))]
        {
            self.0 = unsafe { std::arch::x86_64::_blsr_u64(self.0) }; // Clear the least significant bit
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "bmi1")))]
        {
            self.0 &= self.0 - 1; // Clear the least significant bit
        }
        idx
    }

    #[inline(always)]
    pub fn try_pop_lsb(&mut self) -> Option<u64> {
        if self.0 == 0 {
            return None;
        }
        Some(self.pop_lsb())
    }

    #[inline(always)]
    pub fn msb(&self) -> Option<u64> {
        if self.0 == 0 {
            None
        } else {
            Some(63 - self.0.leading_zeros() as u64)
        }
    }

    #[inline(always)]
    pub const fn const_msb(&self) -> Option<u64> {
        if self.0 == 0 {
            return None;
        }
        Some(63 - self.0.leading_zeros() as u64)
    }

    #[inline(always)]
    pub fn pop_msb(&mut self) -> Option<u64> {
        if self.0 == 0 {
            return None;
        }
        let idx = 63 - self.0.leading_zeros();
        self.0 &= !(1u64 << idx); // Clear the most significant bit
        Some(idx as u64)
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub const fn any(&self) -> bool {
        self.0 != 0
    }

    #[inline(always)]
    pub const fn iter_bits(&self) -> BitBoardIterator {
        BitBoardIterator { remaining: self.0 }
    }

    #[inline(always)]
    pub const fn or(self, rhs: Self) -> Self {
        BitBoard(self.0 | rhs.0)
    }

    #[inline(always)]
    pub const fn and(self, rhs: Self) -> Self {
        BitBoard(self.0 & rhs.0)
    }

    #[inline(always)]
    pub const fn not(self) -> Self {
        BitBoard(!self.0)
    }

    #[inline(always)]
    pub const fn contains_square(&self, index: usize) -> bool {
        (self.0 & (1 << index)) != 0
    }

    #[inline(always)]
    pub fn get_closest_bit(&self, forward: bool) -> Option<u64> {
        if self.is_empty() {
            None
        } else if forward {
            self.lsb()
        } else {
            self.msb()
        }
    }
}

/// Iterator that yields each set bit position in a BitBoard
pub struct BitBoardIterator {
    remaining: u64,
}

impl Iterator for BitBoardIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let idx = self.remaining.trailing_zeros() as usize;
        #[cfg(all(target_arch = "x86_64", target_feature = "bmi1"))]
        {
            self.remaining = unsafe { std::arch::x86_64::_blsr_u64(self.remaining) }; // Clear the least significant bit
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "bmi1")))]
        {
            self.remaining &= self.remaining - 1; // Clear the least significant bit
        }
        Some(idx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact = self.remaining.count_ones() as usize;
        (exact, Some(exact))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.remaining.count_ones() as usize
    }
}

impl ExactSizeIterator for BitBoardIterator {
    fn len(&self) -> usize {
        self.remaining.count_ones() as usize
    }
}

#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub enum Side {
    #[default]
    White,
    Black,
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

    pub const fn white() -> usize {
        Side::White.index()
    }
    pub const fn black() -> usize {
        Side::Black.index()
    }
    pub const fn flip(&self) -> Self {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }
    pub const fn index(&self) -> usize {
        match self {
            Side::White => 0,
            Side::Black => 1,
        }
    }
}

#[derive(Default, PartialEq, Eq, Debug, PartialOrd, Clone, Copy, Hash)]
pub enum Piece {
    #[default]
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}
impl Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Piece::Pawn => write!(f, "Pawn"),
            Piece::Knight => write!(f, "Knight"),
            Piece::Bishop => write!(f, "Bishop"),
            Piece::Rook => write!(f, "Rook"),
            Piece::Queen => write!(f, "Queen"),
            Piece::King => write!(f, "King"),
        }
    }
}
impl Piece {
    pub const PIECES: [Piece; 6] = [
        Piece::Pawn,
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ];

    pub const PIECE_CHARS: [[char; 6]; 2] = [
        ['P', 'N', 'B', 'R', 'Q', 'K'], // White
        ['p', 'n', 'b', 'r', 'q', 'k'], // Black
    ];
    const SIDES: [Side; 2] = [Side::White, Side::Black];
    pub fn all() -> impl Iterator<Item = (Piece, Side)> {
        Self::SIDES
            .iter()
            .flat_map(move |&side| Self::PIECES.iter().map(move |&piece| (piece, side)))
    }

    pub const fn king() -> usize {
        Piece::King.index()
    }
    pub const fn queen() -> usize {
        Piece::Queen.index()
    }
    pub const fn rook() -> usize {
        Piece::Rook.index()
    }
    pub const fn bishop() -> usize {
        Piece::Bishop.index()
    }
    pub const fn knight() -> usize {
        Piece::Knight.index()
    }
    pub const fn pawn() -> usize {
        Piece::Pawn.index()
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
                if stm == Side::White {
                    '♟'
                } else {
                    '♙'
                }
            }
            Piece::Bishop => {
                if stm == Side::White {
                    '♝'
                } else {
                    '♗'
                }
            }
            Piece::Knight => {
                if stm == Side::White {
                    '♞'
                } else {
                    '♘'
                }
            }
            Piece::Rook => {
                if stm == Side::White {
                    '♜'
                } else {
                    '♖'
                }
            }
            Piece::Queen => {
                if stm == Side::White {
                    '♛'
                } else {
                    '♕'
                }
            }
            Piece::King => {
                if stm == Side::White {
                    '♚'
                } else {
                    '♔'
                }
            }
        }
    }

    #[inline(always)]
    pub const fn phase(&self) -> i32 {
        match self {
            Piece::Pawn => 0,
            Piece::Bishop => 1,
            Piece::Knight => 1,
            Piece::Rook => 2,
            Piece::Queen => 4,
            Piece::King => 0,
        }
    }

    #[inline(always)]
    pub const fn score(&self) -> Score {
        match self {
            Piece::Pawn => Score::new(82, 94),
            Piece::Knight => Score::new(337, 281),
            Piece::Bishop => Score::new(365, 297),
            Piece::Rook => Score::new(477, 512),
            Piece::Queen => Score::new(1025, 936),
            Piece::King => Score::new(20_000, 20_000),
        }
    }

    #[inline(always)]
    pub const fn index(&self) -> usize {
        match self {
            Piece::Pawn => 0,
            Piece::Knight => 1,
            Piece::Bishop => 2,
            Piece::Rook => 3,
            Piece::Queen => 4,
            Piece::King => 5,
        }
    }

    #[inline(always)]
    pub const fn victim_score(&self) -> i32 {
        match self {
            Piece::Pawn => 100,
            Piece::Knight => 325,
            Piece::Bishop => 325,
            Piece::Rook => 500,
            Piece::Queen => 1000,
            Piece::King => 20_000,
        }
    }
}

/// Compact struct to hold piece and side
#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct PieceInfo {
    pub piece: Piece,
    pub side: Side,
}

impl PieceInfo {
    pub fn new(piece: Piece, side: Side) -> Self {
        Self { piece, side }
    }
}

/// Bit Boards use 64 bits of true or false, to tell if a given peice is at the location.
/// 12 Bit boards represent where the chess peices are at all times
/// Snapshot of current board
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct BoardState {
    /// Boards for all peices of white and black sides
    all_sides: [BitBoard; 2],
    /// Boards for all peices, of both colors
    /// [Pawn, Bishop, Knight, Rook, Queen, King]
    all_pieces: [[BitBoard; 6]; 2],
    /// Mailbox for fast-lookup. Maps square to piece info
    mailbox: [Option<PieceInfo>; 64],
}

impl Default for BoardState {
    fn default() -> Self {
        Self {
            all_sides: [BitBoard::default(); 2],
            all_pieces: [[BitBoard::default(); 6]; 2],
            mailbox: [None; 64],
        }
    }
}

impl BoardState {
    pub fn to_fen_pieces(&self) -> String {
        let mut fen = String::new();

        for rank in (0..8).rev() {
            let mut empty_count = 0;
            for file in 0..8 {
                let square_index = rank * 8 + file;
                let square = Square::new(square_index).unwrap();

                if let Some((piece, side)) = self.get_piece_at(&square) {
                    if empty_count > 0 {
                        fen.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    let piece_char = Piece::PIECE_CHARS[side.index()][piece.index()];
                    fen.push(piece_char);
                } else {
                    empty_count += 1;
                }
            }

            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
            }

            if rank > 0 {
                fen.push('/');
            }
        }

        fen
    }

    #[inline(always)]
    pub const fn mailbox(&self) -> &[Option<PieceInfo>; 64] {
        &self.mailbox
    }

    #[inline(always)]
    pub const fn get_piece_bb(&self, side: Side, piece: Piece) -> &BitBoard {
        &self.all_pieces[side.index()][piece.index()]
    }

    #[inline(always)]
    pub const fn get_piece_bb_mut(&mut self, side: Side, piece: Piece) -> &mut BitBoard {
        &mut self.all_pieces[side.index()][piece.index()]
    }

    #[inline(always)]
    pub const fn get_colored_pieces(&self, side: Side) -> &[BitBoard; 6] {
        &self.all_pieces[side.index()]
    }

    #[inline(always)]
    pub const fn get_colored_pieces_mut(&mut self, side: Side) -> &mut [BitBoard; 6] {
        &mut self.all_pieces[side.index()]
    }

    #[inline(always)]
    pub const fn get_side_bb(&self, side: Side) -> &BitBoard {
        &self.all_sides[side.index()]
    }

    #[inline(always)]
    pub const fn get_side_bb_mut(&mut self, side: Side) -> &mut BitBoard {
        &mut self.all_sides[side.index()]
    }

    #[inline(always)]
    pub const fn get_orhto_sliders_bb(&self, side: Side) -> BitBoard {
        self.all_pieces[side.index()][Piece::rook()]
            .or(self.all_pieces[side.index()][Piece::queen()])
    }

    #[inline(always)]
    pub const fn get_diag_sliders_bb(&self, side: Side) -> BitBoard {
        self.all_pieces[side.index()][Piece::bishop()]
            .or(self.all_pieces[side.index()][Piece::queen()])
    }

    #[inline(always)]
    pub const fn square_belongs_to(&self, side: Side, square: usize) -> bool {
        self.all_sides[side.index()].contains_square(square)
    }

    #[inline(always)]
    pub const fn is_occupied(&self, square: usize) -> bool {
        self.all_sides[Side::White.index()].contains_square(square)
            || self.all_sides[Side::Black.index()].contains_square(square)
    }

    pub fn set(
        &mut self,
        side_to_set: Side,
        piece_to_set: Piece,
        index_to_set: usize,
    ) -> miette::Result<()> {
        miette::ensure!(
            self.get_piece_at(&index_to_set.into()).is_none(),
            "[set] Some piece already exists at from ( {} ) square",
            Square::new(index_to_set).unwrap()
        );
        self.all_pieces[side_to_set.index()][piece_to_set.index()].set(index_to_set);
        self.all_sides[side_to_set.index()].set(index_to_set);
        self.mailbox[index_to_set] = Some(PieceInfo::new(piece_to_set, side_to_set));
        Ok(())
    }

    pub fn remove_piece(
        &mut self,
        side_to_capture: Side,
        piece_to_capture: Piece,
        index_to_capture: usize,
    ) -> miette::Result<()> {
        miette::ensure!(
            self.get_piece_at(&index_to_capture.into()).is_some(),
            "[capture] No {piece_to_capture} piece at from ( {} ) square",
            Square::new(index_to_capture).unwrap()
        );

        self.all_pieces[side_to_capture.index()][piece_to_capture.index()]
            .capture(index_to_capture);
        self.all_sides[side_to_capture.index()].capture(index_to_capture);
        self.mailbox[index_to_capture] = None;
        Ok(())
    }

    /// Primary way to make moves.
    /// This does NOT handle captures
    pub fn move_piece(&mut self, from: Square, to: Square) -> miette::Result<()> {
        let from_index = from.index();
        let to_index = to.index();

        let piece_info = self.mailbox[from_index]
            .with_context(|| "[move_piece] No piece at from ({from}) square")?;

        let side = piece_info.side;
        let side_index = piece_info.side.index();
        let piece = piece_info.piece;

        miette::ensure!(
            self.get_piece_at(&to).is_none(),
            "[move_piece] Destination square {to} is not empty. Found: {:?}",
            self.get_piece_at(&to)
        );

        // Update piece bitboard
        self.all_pieces[side_index][piece.index()].capture(from_index);
        self.all_pieces[side_index][piece.index()].set(to_index);

        // Update side bitboard
        self.all_sides[side_index].capture(from_index);
        self.all_sides[side_index].set(to_index);

        // Update mailbox
        self.mailbox[from_index] = None;
        self.mailbox[to_index] = Some(PieceInfo::new(piece, side));

        Ok(())
    }

    #[inline(always)]
    pub fn get_piece_at(&self, square: &Square) -> Option<(Piece, Side)> {
        self.mailbox[square.index()].map(|info| (info.piece, info.side))
    }

    #[inline(always)]
    pub fn get_occupied_bb(&self) -> BitBoard {
        self.all_sides[Side::White.index()] | self.all_sides[Side::Black.index()]
    }
}

/// Castling rights are stored in a [`u8`], which is divided into the following parts:
/// ```text
/// Bit: 7 6 5 4 3 2 1 0
///      - - B W q k Q K
///      | | | | | | | |
///      | | | | | | | +-- White kingside right
///      | | | | | | +---- White queenside right
///      | | | | | +------ Black kingside right
///      | | | | +-------- Black queenside right
///      | | | +---------- White has castled
///      | | +------------ Black has castled
///      | +-------------- (unused)
///      +---------------- (unused)
/// ```
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
#[repr(transparent)]
pub struct CastlingRights(pub u8);

impl CastlingRights {
    pub const NO_CASTLING: u8 = 0;
    /// White King side castling
    pub const WHITE_00: u8 = 0b00000001;
    /// White Queen side castling
    pub const WHITE_000: u8 = 0b00000010;
    /// Black King side castling
    pub const BLACK_00: u8 = 0b00000100;
    /// Black Queen side castling
    pub const BLACK_000: u8 = 0b00001000;
    /// White King has castled
    pub const WHITE_CASTLED: u8 = 0b00010000;
    /// Black King has castled
    pub const BLACK_CASTLED: u8 = 0b00100000;

    pub const KING_SIDE: Self = Self(Self::BLACK_00 | Self::WHITE_00);
    pub const QUEEN_SIDE: Self = Self(Self::BLACK_000 | Self::WHITE_000);
    pub const WHITE_CASTLING: Self = Self(Self::WHITE_00 | Self::WHITE_000);
    pub const BLACK_CASTLING: Self = Self(Self::BLACK_00 | Self::BLACK_000);
    pub const ANY_CASTLING: Self = Self(Self::BLACK_CASTLING.0 | Self::WHITE_CASTLING.0);
    pub const RIGHTS_MASK: u8 = 0b00001111;

    #[inline(always)]
    pub const fn add_right(&mut self, rights: CastlingRights) {
        self.0 |= rights.0;
    }
    #[inline(always)]
    pub const fn all() -> Self {
        Self::ANY_CASTLING
    }
    #[inline(always)]
    pub const fn allows(&self, rights: CastlingRights) -> bool {
        self.0 & rights.0 != Self::NO_CASTLING
    }
    #[inline(always)]
    pub const fn get_rights(&self) -> u8 {
        self.0 & Self::RIGHTS_MASK
    }
    #[inline(always)]
    pub const fn can_castle(&self, side: Side, kingside: bool) -> bool {
        match (side, kingside) {
            (Side::White, true) => self.allows(CastlingRights(CastlingRights::WHITE_00)),
            (Side::White, false) => self.allows(CastlingRights(CastlingRights::WHITE_000)),
            (Side::Black, true) => self.allows(CastlingRights(CastlingRights::BLACK_00)),
            (Side::Black, false) => self.allows(CastlingRights(CastlingRights::BLACK_000)),
        }
    }
    #[inline(always)]
    pub const fn set_castled(&mut self, side: Side) {
        match side {
            Side::White => {
                self.0 |= Self::WHITE_CASTLED;
                self.remove_right(&CastlingRights::WHITE_CASTLING);
            }
            Side::Black => {
                self.0 |= Self::BLACK_CASTLED;
                self.remove_right(&CastlingRights::BLACK_CASTLING);
            }
        }
    }
    #[inline(always)]
    pub const fn has_castled(&self, side: Side) -> bool {
        match side {
            Side::White => self.0 & Self::WHITE_CASTLED != 0,
            Side::Black => self.0 & Self::BLACK_CASTLED != 0,
        }
    }
    #[inline(always)]
    pub const fn unset_castled(&mut self, side: Side) {
        match side {
            Side::White => self.0 &= !Self::WHITE_CASTLED,
            Side::Black => self.0 &= !Self::BLACK_CASTLED,
        }
    }
    #[inline(always)]
    pub const fn empty() -> Self {
        Self(Self::NO_CASTLING)
    }
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0b0000
    }
    #[inline(always)]
    pub const fn king_side() -> Self {
        Self::KING_SIDE
    }
    #[inline(always)]
    pub const fn queen_side() -> Self {
        Self::QUEEN_SIDE
    }
    #[inline(always)]
    pub const fn remove_right(&mut self, rights: &CastlingRights) {
        self.0 &= !rights.0
    }
    #[inline(always)]
    pub const fn white_only() -> Self {
        Self::WHITE_CASTLING
    }
    #[inline(always)]
    pub const fn black_only() -> Self {
        Self::BLACK_CASTLING
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
#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(transparent)]
pub struct Square(usize);
impl Square {
    /// Returns a Square from a given index. Will return None if index is out of bounds
    /// index should be [0, 63]
    #[inline(always)]
    pub const fn new(index: usize) -> Option<Self> {
        if index < 64 {
            return Some(Self(index));
        }
        None
    }

    /// Returns a Square from a given File and Rank.
    /// Will return None if either File or Rank are out of bounds.
    /// Rank < 7, File < 8
    #[inline(always)]
    pub const fn from_coords(file: usize, rank: usize) -> Option<Self> {
        if file < 8 && rank < 8 {
            return Some(Square(rank * 8 + file));
        }
        None
    }

    pub fn enpassant_from_index(file: char, rank: char) -> miette::Result<Self> {
        let file = file.to_ascii_lowercase();
        if !('a'..='h').contains(&file) {
            return Err(
                miette::Error::msg("given file isn't valid. Valid file = ['a'..='h']")
                    .context(format!("input file: {file}")),
            );
        }
        if rank != '3' && rank != '6' {
            return Err(
                miette::Error::msg("given rank isn't valid. Valid rank = '3' or '6'")
                    .context(format!("input rank: {rank}")),
            );
        }
        let col_index = file as usize - 'a' as usize;
        let row_index = if rank == '3' { 2 } else { 5 };
        let square_index = row_index * 8 + col_index;
        Ok(Square(square_index))
    }

    #[inline(always)]
    pub const fn coords(&self) -> (usize, usize) {
        let rank = self.0 / 8;
        let file = self.0 % 8;
        (rank, file)
    }

    #[inline(always)]
    pub const fn get_neighbor(&self, dir: Direction) -> Square {
        Self((self.0 as i8 + dir.value()) as usize)
    }

    #[inline(always)]
    pub const fn row(&self) -> usize {
        self.0 / 8
    }

    #[inline(always)]
    pub const fn col(&self) -> usize {
        self.0 % 8
    }

    /// NOTE: Rank is 1 indexed
    #[inline(always)]
    pub const fn rank(&self) -> usize {
        self.0 / 8 + 1
    }

    /// NOTE: File is 1 indexed
    #[inline(always)]
    pub const fn file(&self) -> usize {
        self.0 % 8 + 1
    }

    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.0
    }
}

impl From<Square> for usize {
    fn from(value: Square) -> Self {
        value.0
    }
}

impl From<usize> for Square {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl TryFrom<String> for Square {
    type Error = miette::Report;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Square::from_str(&value)
    }
}

impl FromStr for Square {
    type Err = miette::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        miette::ensure!(
            s.len() == 2,
            "Square needs 1 Letter and 1 Number to construct"
        );
        let s = s.to_ascii_uppercase();
        let mut iter = s.chars();
        let letter = iter.next().context("1st char should be letter")?;
        let num = iter.next().context("2nd char should be number")?;
        let file = letter as u8 - b'A';
        let rank = num as u8 - b'1';

        let idx = (8 * rank + file) as usize;
        Ok(Self(idx))
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
    use crate::{board::Board, moves::move_info::Move, utils::log::init};

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
        init();
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
        let mov = Move::new(8, 16, Move::QUIET);
        assert!(board.try_move(mov).is_ok());
        let o = board.positions.get_side_bb(Side::White).print_bitboard();
        assert_eq!(out, o);
    }
}
