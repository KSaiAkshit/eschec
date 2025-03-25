use std::collections::HashMap;

use self::components::{CastlingRights, Pieces, Position, Side, Square};

pub mod components;
pub mod fen;

/// Completely encapsulate the game
#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Board {
    // Snapshot of current board
    pub positions: Position,
    /// Side to move, 0 - white, 1 - black
    pub stm: Side,
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
        // const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let mut board = Self {
            positions: Position::default(),
            stm: Side::default(),
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
            ('P', (Pieces::PAWN, Side::White as usize)),
            ('p', (Pieces::PAWN, Side::Black as usize)),
            ('B', (Pieces::BISHOP, Side::White as usize)),
            ('b', (Pieces::BISHOP, Side::Black as usize)),
            ('N', (Pieces::KNIGHT, Side::White as usize)),
            ('n', (Pieces::KNIGHT, Side::Black as usize)),
            ('R', (Pieces::ROOK, Side::White as usize)),
            ('r', (Pieces::ROOK, Side::Black as usize)),
            ('Q', (Pieces::QUEEN, Side::White as usize)),
            ('q', (Pieces::QUEEN, Side::Black as usize)),
            ('K', (Pieces::KING, Side::White as usize)),
            ('k', (Pieces::KING, Side::Black as usize)),
        ]
        .into_iter()
        .collect();
        // rank [7,0]
        let mut rank = 7;
        // file [0,7]
        let mut file = 0;
        // dbg!(rank, file);
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
