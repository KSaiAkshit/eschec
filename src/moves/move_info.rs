use std::{fmt::Display, str::FromStr};

use miette::Context;

use crate::{Board, CastlingRights, Piece, Square};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveInfo {
    pub from: Square,
    pub to: Square,
    pub piece_moved: Piece,
    pub captured_piece: Option<Piece>,
    pub promotion: Option<Piece>,
    pub is_castling: bool,
    pub is_en_passant: bool,
    pub castle_rights: CastlingRights,    // prev
    pub enpassant_square: Option<Square>, // prev
    pub halfmove_clock: u8,               // prev
    pub zobrist_hash: u64,                // prev
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
        Square::new((self.0 & Self::FROM_MASK) as usize).unwrap()
    }

    /// Extract the to-square index (0..63)
    #[inline]
    pub const fn to_sq(&self) -> Square {
        Square::new(((self.0 & Self::TO_MASK) >> 6) as usize).unwrap()
    }

    /// Extract the flags (upper 4 bits)
    #[inline]
    pub const fn flags(&self) -> u16 {
        self.0 & Self::FLAG_MASK
    }

    /// Returns true if this move is a capture (including en passant and promotion-capture)
    #[inline]
    pub const fn is_capture(&self) -> bool {
        matches!(
            self.flags(),
            Self::CAPTURE
                | Self::EN_PASSANT
                | Self::PROMO_NC
                | Self::PROMO_BC
                | Self::PROMO_RC
                | Self::PROMO_QC
        )
    }

    /// Returns true if this move is a promotion
    #[inline]
    pub const fn is_promotion(&self) -> bool {
        (self.flags() >> 12) >= 0b1000
    }

    /// Returns true if this move is a castling
    #[inline]
    pub const fn is_castling(&self) -> bool {
        matches!(self.flags(), Self::KING_CASTLE | Self::QUEEN_CASTLE)
    }

    /// Returns true if this move is en_passant
    #[inline]
    pub const fn is_enpassant(&self) -> bool {
        matches!(self.flags(), Self::EN_PASSANT)
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

    pub fn from_uci(board: &Board, uci: &str) -> miette::Result<Move> {
        if uci.len() < 4 || uci.len() > 5 {
            miette::bail!("Invalid UCI move format: '{}'", uci);
        }
        let from_str = &uci[0..2];
        let to_str = &uci[2..4];
        let promo_char = uci.chars().nth(4);

        let from = Square::from_str(from_str)?;
        let to = Square::from_str(to_str)?;

        // Find the matching legal move. This is the only way to get the correct flags.
        let legal_moves = board.generate_legal_moves();
        let found_move = legal_moves.into_iter().find(|m| {
            if m.from_sq() == from && m.to_sq() == to {
                // If there's a promotion, make sure it matches.
                if let Some(pc) = promo_char {
                    return m.promoted_piece_char() == Some(pc);
                }
                // If no promotion in UCI string, match a non-promotion move.
                return !m.is_promotion();
            }
            false
        });

        found_move.context(format!(
            "The move '{uci}' is not legal in the current position."
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
