use crate::{CastlingRights, Piece, Square};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
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
}

impl Move {
    pub fn new(from: Square, to: Square) -> Self {
        Self {
            from,
            to,
            ..Default::default()
        }
    }
}
