use std::{collections::HashMap, fmt::Display, ops::BitOr};

#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct BitBoard(pub u64);

impl BitOr for BitBoard {
    type Output = BitBoard;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitBoard {
    fn set(&mut self, position: usize) {
        // dbg!(position);
        let mask = 1 << position;
        self.0 ^= mask;
    }
}

pub struct Sides;
impl Sides {
    pub const WHITE: usize = 0;
    pub const BLACK: usize = 1;
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

/// Represents a single square on the board.
/// # Representation
/// 1 is A1
/// 2 is B1
/// 64 is H8
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Square(usize);

/// Completely encapsulate the game
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Board {
    // Snapshot of current board
    pub positions: Position,
    /// Side to move, 0 - white, 1 - black
    pub stm: usize,
    /// Castling rights for both sides, KQ - White king&queen side, kq - Black king&queen side, '-' no right
    pub castling_rights: CastlingRights,
    /// En passant target square
    pub enpassant_square: Option<Square>,
    /// Specifies a number of half-moves with respect to the 50 move draw rule. It is reset(0) after a capture or a pawn move and incremented otherwise.
    pub halfmove_clock: u8,
    ///  The number of the full moves in a game. It starts at 1, and is incremented after each Black's move.
    pub fullmove_counter: u8,
}

impl Board {
    pub fn new() -> Self {
        const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
        let mut board = Self {
            positions: Position::default(),
            stm: usize::default(),
            castling_rights: CastlingRights::default(),
            enpassant_square: Option::default(),
            halfmove_clock: u8::default(),
            fullmove_counter: u8::default(),
        };
        match board.place_pieces(START_FEN) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error initializing board: {}", e);
            }
        }
        board
    }
    /*
    Parse the FEN string to extract the piece placement part. This part of the FEN string represents the positions of the pieces on the board.
    Iterate over each character of the piece placement part of the FEN string.
    Map each character to the corresponding piece type and side using a lookup table. For example, 'P' represents a white pawn, 'p' represents a black pawn, 'K' represents a white king, and so on.
    Determine the corresponding position on the board for each piece based on its rank and file. The rank and file are represented by the row and column of the chessboard, respectively.
    Update the bb_pieces array in the Position struct to reflect the positions of the pieces for each side. You'll need to update the appropriate BitBoard for each piece type and side.
    Ensure that the bb_sides array in the Position struct is updated accordingly to reflect the presence of pieces on each side of the board.
    Initialize the Board struct with the Position struct containing the updated piece positions.    */
    pub fn place_pieces(&mut self, fen: &str) -> anyhow::Result<()> {
        // TODO: Allow for full FEN notation
        if fen.contains(' ') {
            return Err(anyhow::Error::msg("Not supported for now"));
        }
        let lookup_table: HashMap<char, (usize, usize)> = [
            ('P', (Pieces::PAWN, Sides::WHITE)),
            ('p', (Pieces::PAWN, Sides::BLACK)),
            ('B', (Pieces::BISHOP, Sides::WHITE)),
            ('b', (Pieces::BISHOP, Sides::BLACK)),
            ('N', (Pieces::KNIGHT, Sides::WHITE)),
            ('n', (Pieces::KNIGHT, Sides::BLACK)),
            ('R', (Pieces::ROOK, Sides::WHITE)),
            ('r', (Pieces::ROOK, Sides::BLACK)),
            ('Q', (Pieces::QUEEN, Sides::WHITE)),
            ('q', (Pieces::QUEEN, Sides::BLACK)),
            ('K', (Pieces::KING, Sides::WHITE)),
            ('k', (Pieces::KING, Sides::BLACK)),
        ]
        .into_iter()
        .collect();
        let mut rank = 7; // rank [7,0]
        let mut file = 0; // file [0,7]
        for c in fen.chars() {
            // dbg!("---------------------");
            // dbg!(c);
            match c {
                '1'..='8' => {
                    // dbg!(rank, file);
                    file += c.to_digit(10).unwrap() as usize - 1;
                    // dbg!(rank, file);
                }
                '/' => {
                    // dbg!(rank, file);
                    rank -= 1;
                    file = 0;
                    // dbg!(rank, file);
                }
                _ => {
                    if let Some((piece, side)) = lookup_table.get(&c) {
                        // dbg!(piece, side);
                        // dbg!(rank, file);
                        self.positions.all_pieces[*side][*piece].set(rank * 8 + file);
                        file += 1;
                    } else {
                        return Err(anyhow::Error::msg("Invalid Fen Character"));
                    }
                }
            }
        }
        // updte all_sides too
        self.positions.all_sides[0] = self.positions.all_pieces[0][0]
            | self.positions.all_pieces[0][1]
            | self.positions.all_pieces[0][2]
            | self.positions.all_pieces[0][3]
            | self.positions.all_pieces[0][4]
            | self.positions.all_pieces[0][5];

        self.positions.all_sides[1] = self.positions.all_pieces[1][0]
            | self.positions.all_pieces[1][1]
            | self.positions.all_pieces[1][2]
            | self.positions.all_pieces[1][3]
            | self.positions.all_pieces[1][4]
            | self.positions.all_pieces[1][5];
        Ok(())
    }
}
impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}
