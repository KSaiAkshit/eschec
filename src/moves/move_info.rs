use std::{fmt::Display, str::FromStr};

use miette::Context;

use crate::prelude::*;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveInfo {
    pub from: Square,
    pub to: Square,
    pub stm: Side, // prev
    pub piece_moved: Piece,
    pub captured_piece: Option<Piece>,
    pub promotion: Option<Piece>,
    pub is_castling: bool,
    pub is_en_passant: bool,
    pub castle_rights: CastlingRights,    // prev
    pub enpassant_square: Option<Square>, // prev
    pub halfmove_clock: u8,               // prev
    pub zobrist_hash: u64,                // prev
                                          // pub material: [Score; 2],             // prev
}

impl MoveInfo {
    pub fn new(from: Square, to: Square) -> Self {
        Self {
            from,
            to,
            ..Default::default()
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Move(pub u16);

impl Move {
    /// Bitflags for move types
    pub const QUIET: u16 = 0b0000 << 12;
    pub const DOUBLE_PAWN: u16 = 0b0001 << 12;
    pub const KING_CASTLE: u16 = 0b0010 << 12;
    pub const QUEEN_CASTLE: u16 = 0b0011 << 12;
    pub const CAPTURE: u16 = 0b0100 << 12;
    pub const EN_PASSANT: u16 = 0b0101 << 12;
    pub const PROMO: u16 = 0b1000 << 12;

    /// Promotions
    pub const PROMO_N: u16 = 0b1000 << 12;
    pub const PROMO_B: u16 = 0b1001 << 12;
    pub const PROMO_R: u16 = 0b1010 << 12;
    pub const PROMO_Q: u16 = 0b1011 << 12;
    pub const PROMO_NC: u16 = 0b1100 << 12;
    pub const PROMO_BC: u16 = 0b1101 << 12;
    pub const PROMO_RC: u16 = 0b1110 << 12;
    pub const PROMO_QC: u16 = 0b1111 << 12;

    /// Bitmask consts
    pub const FLAG_MASK: u16 = 0xF000;
    pub const TO_MASK: u16 = 0x0FC0;
    pub const FROM_MASK: u16 = 0x003F;

    pub const fn new(from: u8, to: u8, flags: u16) -> Self {
        Self((from as u16) | ((to as u16) << 6) | flags)
    }

    /// Extract the from-square index (0..63)
    #[inline]
    pub const fn from_idx(&self) -> u8 {
        (self.0 & Self::FROM_MASK) as u8
    }

    /// Extract the to-square index (0..63)
    #[inline]
    pub const fn to_idx(&self) -> u8 {
        ((self.0 & Self::TO_MASK) >> 6) as u8
    }

    /// Extract the from-square index (0..63)
    #[inline]
    pub const fn from_sq(&self) -> Square {
        // SAFETY: Mask 0x3F guarantees value is 0..63
        unsafe { Square::new_unchecked((self.0 & Self::FROM_MASK) as usize) }
    }

    /// Extract the to-square index (0..63)
    #[inline]
    pub const fn to_sq(&self) -> Square {
        // SAFETY: Mask 0x3F guarantees value is 0..63
        unsafe { Square::new_unchecked(((self.0 & Self::TO_MASK) >> 6) as usize) }
    }

    /// Extract the flags (upper 4 bits)
    #[inline(always)]
    pub const fn flags(&self) -> u16 {
        self.0 & Self::FLAG_MASK
    }

    /// Returns true if this move is a capture (including en passant and promotion-capture)
    #[inline]
    pub const fn is_capture(&self) -> bool {
        (self.0 & Self::CAPTURE) != 0
    }

    /// Returns true if this move is a promotion
    #[inline]
    pub const fn is_promotion(&self) -> bool {
        (self.0 & Self::PROMO) != 0
    }

    /// Returns true if this move is quiet
    #[inline]
    pub const fn is_quiet(&self) -> bool {
        self.flags() == Self::QUIET
    }

    /// Returns true if this move is a castling
    #[inline]
    pub const fn is_castling(&self) -> bool {
        let f = self.flags();
        f == Self::KING_CASTLE || f == Self::QUEEN_CASTLE
    }

    /// Returns true if this move is en_passant
    #[inline]
    pub const fn is_enpassant(&self) -> bool {
        self.flags() == Self::EN_PASSANT
    }

    /// Returns promoted piece type as types, if promotion; else None
    #[inline]
    pub const fn promoted_piece(&self) -> Option<Piece> {
        match self.flags() {
            Self::PROMO_N | Self::PROMO_NC => Some(Piece::Knight),
            Self::PROMO_B | Self::PROMO_BC => Some(Piece::Bishop),
            Self::PROMO_R | Self::PROMO_RC => Some(Piece::Rook),
            Self::PROMO_Q | Self::PROMO_QC => Some(Piece::Queen),
            _ => None,
        }
    }

    /// Returns promoted piece type as char, if promotion; else None
    #[inline]
    pub const fn promoted_piece_char(&self) -> Option<char> {
        match self.flags() {
            Self::PROMO_N | Self::PROMO_NC => Some('n'),
            Self::PROMO_B | Self::PROMO_BC => Some('b'),
            Self::PROMO_R | Self::PROMO_RC => Some('r'),
            Self::PROMO_Q | Self::PROMO_QC => Some('q'),
            _ => None,
        }
    }

    /// Utility: returns a 'e2e4', 'e7e8q' etc
    pub fn uci(&self) -> String {
        let from = Square::new(self.from_idx().into()).unwrap();
        let to = Square::new(self.to_idx().into()).unwrap_or_default();
        let uci_str = match self.promoted_piece_char() {
            Some(piece) => format!("{from}{to}{piece}"),
            None => format!("{from}{to}",),
        };
        uci_str.to_lowercase()
    }

    /// 0..63 to e2 etc
    pub fn square_to_coord(idx: u8) -> String {
        let file = (b'a' + (idx % 8)) as char;
        let rank = (b'1' + (idx / 8)) as char;
        format!("{file}{rank}")
    }

    /// Creates a Move from Standard Algebraic Notation (SAN) given a board state.
    ///
    /// This is more complex than `from_uci` because SAN is context-dependent.
    /// It works by generating all legal moves and finding the one that matches the SAN string.
    ///
    /// # Arguments
    /// * `board` - The board state from which the move is made.
    /// * `san` - The SAN string (e.g., "Nf3", "e4", "O-O", "fxg8=Q+").
    ///
    /// # Returns
    /// A `Result` containing the `Move` if successful, or an error if the
    /// SAN is invalid or doesn't correspond to a legal move.
    pub fn from_san(board: &Board, san: &str) -> miette::Result<Move> {
        let mut legal_moves = MoveBuffer::new();
        board.generate_legal_moves(&mut legal_moves, false);

        // Normalize the SAN string
        let clean_san = san.trim_end_matches(['+', '#']);

        // Handle castling
        if clean_san == "O-O" {
            // Kingside
            return legal_moves
                .iter()
                .find(|m| m.is_castling() && m.to_sq().col() > m.from_sq().col())
                .copied()
                .context(format!(
                    "Kingside castling ('O-O') is not legal in this position: {}",
                    board.to_fen()?
                ));
        }
        if clean_san == "O-O-O" {
            // Queenside
            return legal_moves
                .iter()
                .find(|m| m.is_castling() && m.to_sq().col() < m.from_sq().col())
                .copied()
                .context(format!(
                    "Queenside castling ('O-O-O') is not legal in this position: {}",
                    board.to_fen()?
                ));
        }

        // Parse the components of the SAN move.
        let san_move = SanMove::parse(clean_san)?;

        // Find the single legal move that matches the parsed SAN components.
        let mut matching_move: Option<Move> = None;
        for &legal_move in legal_moves.iter() {
            if san_move.matches(legal_move, board) {
                if matching_move.is_some() {
                    // This indicates the SAN was ambiguous (e.g., "Rd1" when two rooks can move to d1).
                    // A fully compliant parser would handle this, but for test suites this is rare.
                    miette::bail!("Ambiguous SAN move: '{}'", san);
                }
                matching_move = Some(legal_move);
            }
        }

        matching_move.context(format!(
            "The SAN move '{}' is not legal in the current position: {}",
            san,
            board.to_fen()?
        ))
    }

    /// Creates a Move from Universal Chess Interface (UCI) notation.
    ///
    /// This is the most efficient implementation. It constructs the move with the
    /// correct flags based on the board state and then verifies its legality by
    /// making the move on a temporary board and checking if the king is left in check.
    /// It does NOT generate all legal moves.
    ///
    /// # Arguments
    /// * `board` - The board state from which the move is made.
    /// * `uci` - The UCI string (e.g., "e2e4", "g1f3", "a7a8q").
    pub fn from_uci(board: &Board, uci: &str) -> miette::Result<Move> {
        if uci.len() < 4 || uci.len() > 5 {
            miette::bail!("Invalid UCI move format: '{}'", uci);
        }

        let from = Square::from_str(&uci[0..2])?;
        let to = Square::from_str(&uci[2..4])?;
        let promo_char = uci.chars().nth(4);

        let piece = board.get_piece_at(from).context(format!(
            "No piece at the 'from' square '{}' in UCI move '{}'",
            from, uci
        ))?;

        let mut flags = Move::QUIET;
        if piece == Piece::Pawn {
            let promo_rank = if board.stm == Side::White { 7 } else { 0 };
            if to.row() == promo_rank {
                let promotion_piece = match promo_char {
                    Some('q') => Piece::Queen,
                    Some('r') => Piece::Rook,
                    Some('b') => Piece::Bishop,
                    Some('n') => Piece::Knight,
                    None => miette::bail!("Promotion move '{}' is missing a promotion piece", uci),
                    _ => miette::bail!("Invalid promotion piece in UCI move '{}'", uci),
                };
                let is_capture = board.get_piece_at(to).is_some();
                flags = match (promotion_piece, is_capture) {
                    (Piece::Queen, true) => Move::PROMO_QC,
                    (Piece::Queen, false) => Move::PROMO_Q,
                    (Piece::Rook, true) => Move::PROMO_RC,
                    (Piece::Rook, false) => Move::PROMO_R,
                    (Piece::Bishop, true) => Move::PROMO_BC,
                    (Piece::Bishop, false) => Move::PROMO_B,
                    (Piece::Knight, true) => Move::PROMO_NC,
                    (Piece::Knight, false) => Move::PROMO_N,
                    _ => unreachable!(),
                };
            } else if promo_char.is_some_and(|c| ['q', 'r', 'b', 'n'].contains(&c)) {
                miette::bail!(
                    "Invalid promotion rank (not 2nd or 7th rank) in UCI move: {}",
                    to.rank()
                );
            }
        }
        if flags == Move::QUIET {
            if piece == Piece::King {
                if (from.col() as i8 - to.col() as i8).abs() == 2 {
                    flags = if to.col() > from.col() {
                        Move::KING_CASTLE
                    } else {
                        Move::QUEEN_CASTLE
                    };
                }
            } else if piece == Piece::Pawn {
                if let Some(ep_square) = board.enpassant_square
                    && to == ep_square
                    && (from.col() as i8 - to.col() as i8).abs() == 1
                {
                    flags = Move::EN_PASSANT;
                }
                if (from.row() as i8 - to.row() as i8).abs() == 2 {
                    flags = Move::DOUBLE_PAWN;
                }
            }
        }
        if flags == Move::QUIET && board.get_piece_at(to).is_some() {
            flags = Move::CAPTURE;
        }
        let candidate_move = Move::new(from.index() as u8, to.index() as u8, flags);

        let mut temp_board = *board;
        // make_move might or might not fail gracefully.
        // Need to generate piecewise legal moves for completeness
        if temp_board.make_move(candidate_move).is_ok() {
            // The move was made. Now check if the king of the side that *just moved* is in check.
            // The side to move has flipped inside `make_move`, so we check the *opposite* of the new `stm`.
            if !temp_board.is_in_check(board.stm) {
                return Ok(candidate_move);
            }
        }

        Err(miette::miette!(
            "The move '{}' is not legal in the current position.",
            uci
        ))
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = self.0;
        let flags = (val >> 12) & 0xF;
        let to_square = (val >> 6) & 0x3F;
        let from_square = val & 0x3F;

        writeln!(f, "Flags: {flags:04b}")?;
        writeln!(f, "To square: {to_square}")?;
        writeln!(f, "From square: {from_square}")?;

        Ok(())
    }
}

/// A helper struct to represent the components of a parsed SAN move.
struct SanMove {
    piece: Piece,
    to: Square,
    from_file: Option<usize>, // 1-indexed file
    from_rank: Option<usize>, // 1-indexed rank
    is_capture: bool,
    promotion: Option<Piece>,
}

impl SanMove {
    /// Parses a cleaned SAN string (no checks/mates) into its components.
    fn parse(san: &str) -> miette::Result<Self> {
        let original_san = san.to_string();

        // 1. Check for promotion (same as before)
        let promotion = match san.chars().next_back() {
            Some(p_char)
                if "QRBN".contains(p_char) && san.ends_with(p_char) && san.contains('=') =>
            {
                Some(match p_char {
                    'Q' => Piece::Queen,
                    'R' => Piece::Rook,
                    'B' => Piece::Bishop,
                    'N' => Piece::Knight,
                    _ => unreachable!(),
                })
            }
            _ => None,
        };
        let san_no_promo = if promotion.is_some() {
            &san[..san.len() - 2]
        } else {
            san
        };

        // 2. Identify destination square (same as before)
        if san_no_promo.len() < 2 {
            miette::bail!("Invalid SAN string: '{}'", original_san);
        }
        let to_str = &san_no_promo[san_no_promo.len() - 2..];
        let to = Square::from_str(to_str)?;

        // 3. The part before the destination square
        let prefix = &san_no_promo[..san_no_promo.len() - 2];
        let mut prefix_chars = prefix.chars();

        // 4. Identify the piece
        let piece = match prefix_chars.next() {
            Some(c) if "KQRBN".contains(c) => match c {
                'K' => Piece::King,
                'Q' => Piece::Queen,
                'R' => Piece::Rook,
                'B' => Piece::Bishop,
                'N' => Piece::Knight,
                _ => unreachable!(),
            },
            _ => Piece::Pawn,
        };

        // 5. The rest of the prefix is for capture and disambiguation
        let from_info_str = if piece == Piece::Pawn {
            prefix
        } else {
            prefix_chars.as_str()
        };

        let is_capture = from_info_str.contains('x');
        let from_info = from_info_str.replace('x', "");

        let mut from_file = None;
        let mut from_rank = None;

        for c in from_info.chars() {
            if ('a'..='h').contains(&c) {
                from_file = Some((c as usize) - ('a' as usize) + 1);
            } else if ('1'..='8').contains(&c) {
                from_rank = Some(c.to_digit(10).unwrap() as usize);
            }
        }

        Ok(Self {
            piece,
            to,
            from_file,
            from_rank,
            is_capture,
            promotion,
        })
    }

    /// Checks if a given legal `Move` matches the parsed SAN components.
    fn matches(&self, legal_move: Move, board: &Board) -> bool {
        if self.to != legal_move.to_sq() {
            return false;
        }
        if self.promotion != legal_move.promoted_piece() {
            return false;
        }

        let from_sq = legal_move.from_sq();
        if board.get_piece_at(from_sq) != Some(self.piece) {
            return false;
        }

        // For pawn moves, the capture flag must match exactly.
        // For other pieces, a move to an enemy square is a capture.
        if self.piece == Piece::Pawn {
            if self.is_capture != legal_move.is_capture() {
                return false;
            }
        } else {
            // If SAN indicates a capture, the move must be a capture.
            // If SAN does not, it could still be a capture (e.g., "Rd8" capturing a piece on d8).
            // The presence of 'x' is for disambiguation, not just flagging a capture.
            // However, for test suites, a simple check is usually enough.
            if self.is_capture && !legal_move.is_capture() {
                return false;
            }
        }

        if let Some(file) = self.from_file
            && from_sq.file() != file
        {
            return false;
        }
        if let Some(rank) = self.from_rank
            && from_sq.rank() != rank
        {
            return false;
        }

        true
    }
}
