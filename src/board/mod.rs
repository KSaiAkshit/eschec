use std::collections::HashMap;

use moves::Moves;

use self::components::{BoardState, CastlingRights, Piece, Side, Square};

pub mod components;
mod fen;
pub mod moves;

/// Completely encapsulate the game
#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Board {
    // Snapshot of current board
    pub positions: BoardState,
    /// Side to move, 0 - white, 1 - black
    pub stm: Side,
    /// Castling rights for both sides, KQ - White king&queen side, kq - Black king&queen side, '-' no right
    pub castling_rights: CastlingRights,
    /// En passant target square
    pub enpassant_square: Option<Square>,
    /// Specifies a number of half-moves with respect to the 50 move draw rule. It is reset(to 0) after a capture
    /// or a pawn move and incremented otherwise.
    pub halfmove_clock: u8,
    ///  The number of the full moves in a game. It starts at 1, and is incremented after each Black's move.
    pub fullmove_counter: u8,
}

impl Board {
    /// Use to initialize a default board
    pub fn new() -> Self {
        // const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
        const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let mut board = Self {
            positions: BoardState::default(),
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
    /// Use this to construct a board from fen
    pub fn from_fen(fen: &str) -> Self {
        let parsed = fen::parse_fen(fen);
        match parsed {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Got error while parsing given fen: {}", e);
                panic!("very bad fen")
            }
        }
    }

    pub fn generate_legal_moves(&self) -> Vec<(Square, Square)> {
        let mut legal_moves = Vec::new();
        (0..64).for_each(|index| {
            let square = Square::new(index).expect("Get a valid index");
            if let Some(piece) = self.get_piece_at(square) {
                let moves = Moves::new(piece);
                moves.attack_bb.into_iter().for_each(|attack_bb| {
                    let targets = attack_bb.get_set_bits();
                    targets.into_iter().for_each(|target_index| {
                        let target_square = Square::new(target_index).expect("Get a valid index");
                        if self.is_move_legal(square, target_square) {
                            legal_moves.push((square, target_square));
                        }
                    });
                });
            }
        });
        legal_moves
    }

    pub fn make_move(&mut self, from: Square, to: Square) -> anyhow::Result<()> {
        if !self.is_move_legal(from, to) {
            anyhow::bail!("Illegal move from {} to {}", from, to);
        }
        if let Some(piece) = self.get_piece_at(from) {
            self.positions
                .update_piece_position(&piece, &self.stm, from, to);

            self.handle_special_rules(from, to);
            self.stm = self.stm.flip();
            self.halfmove_clock += 1;
            if self.stm == Side::White {
                self.fullmove_counter += 1;
            }
            Ok(())
        } else {
            anyhow::bail!("No piece at from Square");
        }
    }

    pub fn is_move_legal(&self, from: Square, to: Square) -> bool {
        // 1. Check if there is a piece at the 'from' square
        let piece = match self.get_piece_at(from) {
            Some(p) => p,
            None => return false,
        };

        // 2. Check if the piece belongs to the current side to move
        if !self.positions.all_sides[self.stm.index()].contains_square(from.index()) {
            return false;
        }

        // 3. Generate legal moves for the piece
        let moves = Moves::new(piece);
        let legal_squares = moves.attack_bb[from.index()];

        // 4. Check if the 'to' square is a legal square for the piece
        if !legal_squares.contains_square(to.index()) {
            return false;
        }

        // 5. Check if the move puts own king in check
        let mut board = self.clone();
        board.make_move(from, to).unwrap();
        !board.is_in_check(self.stm)
    }

    pub fn is_in_check(&self, side: Side) -> bool {
        // 1. Find king's position
        let king_bb = self.positions.all_pieces[side.index()][Piece::King.index()];
        let king_square =
            Square::new(king_bb.get_set_bits()[0]).expect("Should be able to find king");

        // 2. Generate oppponent's attacks
        let opponent = side.flip();
        for piece in Piece::colored_pieces(opponent) {
            let piece_bb = self.positions.all_pieces[opponent.index()][piece.index()];
            let moves = Moves::new(piece);
            // If any opponent piece can attack king's square, king is in check
            if piece_bb
                .get_set_bits()
                .iter()
                .any(|&from| moves.attack_bb[from].contains_square(king_square.index()))
            {
                return true;
            }
        }

        false
    }

    pub fn is_checkmate(&self, side: Side) -> bool {
        self.is_in_check(side) && !self.generate_legal_moves().is_empty()
    }

    pub fn is_stalemate(&self, side: Side) -> bool {
        !self.is_in_check(side) && self.generate_legal_moves().is_empty()
    }

    pub fn is_draw(&self) -> bool {
        self.is_stalemate(self.stm) || self.halfmove_clock >= 100 || self.is_insufficient_material()
    }

    // NOTE: Older Implementation
    fn place_pieces(&mut self, fen: &str) -> anyhow::Result<()> {
        if fen.contains(' ') {
            return Err(anyhow::Error::msg("Not supported for now"));
        }
        let lookup_table: HashMap<char, (usize, usize)> = [
            ('P', (Piece::Pawn as usize, Side::White as usize)),
            ('p', (Piece::Pawn as usize, Side::Black as usize)),
            ('B', (Piece::Bishop as usize, Side::White as usize)),
            ('b', (Piece::Bishop as usize, Side::Black as usize)),
            ('N', (Piece::Knight as usize, Side::White as usize)),
            ('n', (Piece::Knight as usize, Side::Black as usize)),
            ('R', (Piece::Rook as usize, Side::White as usize)),
            ('r', (Piece::Rook as usize, Side::Black as usize)),
            ('Q', (Piece::Queen as usize, Side::White as usize)),
            ('q', (Piece::Queen as usize, Side::Black as usize)),
            ('K', (Piece::King as usize, Side::White as usize)),
            ('k', (Piece::King as usize, Side::Black as usize)),
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

    pub fn get_piece_at(&self, square: Square) -> Option<Piece> {
        let index = square.index();

        for (piece_type, side) in Piece::all() {
            let piece_bb = self.positions.all_pieces[side as usize][piece_type as usize];
            if piece_bb.contains_square(index) {
                return Some(piece_type);
            }
        }
        None
    }

    pub fn handle_special_rules(&mut self, from: Square, to: Square) {
        // Handle castling
        if let Some(Piece::King) = self.get_piece_at(from) {
            // Update castling rights
            match self.stm {
                Side::White => self
                    .castling_rights
                    .remove_right(CastlingRights::WHITE_CASTLING),
                Side::Black => self
                    .castling_rights
                    .remove_right(CastlingRights::BLACK_CASTLING),
            }
            // Move rook if castling
            // TODO: Implement castling logic
        }

        // Handle en passant
        if let Some(Piece::Pawn) = self.get_piece_at(from) {
            // Set en passant square if double pawn push
            // A double pawn push moves two squares forward from starting position
            let is_double_push = match self.stm {
                Side::White => from.index() / 8 == 1 && to.index() / 8 == 3,
                Side::Black => from.index() / 8 == 6 && to.index() / 8 == 4,
            };

            // If double push detected, set en passant square to the skipped square
            if is_double_push {
                let skipped_square = Square::new(match self.stm {
                    Side::White => from.index() + 8,
                    Side::Black => from.index() - 8,
                })
                .unwrap();
                self.enpassant_square = Some(skipped_square);
            }

            // If moving to en passant square, capture the enemy pawn
            if let Some(ep_square) = self.enpassant_square {
                if to == ep_square {
                    let captured_pawn_square = Square::new(match self.stm {
                        Side::White => ep_square.index() - 8,
                        Side::Black => ep_square.index() + 8,
                    })
                    .unwrap();
                    self.positions.all_pieces[self.stm.flip().index()][Piece::Pawn.index()]
                        .capture(captured_pawn_square.index());
                }
            }
        }
    }

    fn calculate_material(&self) {
        let mut material = 0;
        for piece in self.positions.all_pieces.iter().flatten() {
            material += piece.into();
        }
        self.material = material;
    }

    fn is_insufficient_material(&self) -> bool {
        todo!()
    }
}
