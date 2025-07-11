use std::{collections::HashMap, sync::LazyLock};

use miette::{Context, IntoDiagnostic};

use crate::{BoardState, Piece};

use super::{
    Board,
    components::{CastlingRights, Side, Square},
};
pub static PIECE_CHAR_LOOKUP_TABLE: LazyLock<HashMap<char, (Piece, Side)>> = LazyLock::new(|| {
    [
        ('P', (Piece::Pawn, Side::White)),
        ('p', (Piece::Pawn, Side::Black)),
        ('B', (Piece::Bishop, Side::White)),
        ('b', (Piece::Bishop, Side::Black)),
        ('N', (Piece::Knight, Side::White)),
        ('n', (Piece::Knight, Side::Black)),
        ('R', (Piece::Rook, Side::White)),
        ('r', (Piece::Rook, Side::Black)),
        ('Q', (Piece::Queen, Side::White)),
        ('q', (Piece::Queen, Side::Black)),
        ('K', (Piece::King, Side::White)),
        ('k', (Piece::King, Side::Black)),
    ]
    .into()
});

pub fn to_fen(board: &Board) -> miette::Result<String> {
    let mut fen = String::new();
    let piece_placement: &str = &board.positions.to_fen_pieces();
    let stm: &str = match &board.stm {
        Side::White => "w",
        Side::Black => "b",
    };
    let castling_rights: &str = &board.castling_rights.to_string();
    let enpassent_square: &str = match board.enpassant_square {
        Some(sq) => &sq.to_string().to_ascii_lowercase(),
        None => "-",
    };
    let halfmove_clock: &str = &board.halfmove_clock.to_string();
    let fullmove_clock: &str = &board.fullmove_counter.to_string();

    fen.push_str(piece_placement);
    fen.push(' ');
    fen.push_str(stm);
    fen.push(' ');
    fen.push_str(castling_rights);
    fen.push(' ');
    fen.push_str(enpassent_square);
    fen.push(' ');
    fen.push_str(halfmove_clock);
    fen.push(' ');
    fen.push_str(fullmove_clock);
    Ok(fen)
}

/// Parse the FEN string to extract the piece placement part. This part of the FEN string represents the positions of the pieces on the board.
/// Iterate over each character of the piece placement part of the FEN string.
/// Map each character to the corresponding piece type and side using a lookup table. For example,
/// 'P' represents a white pawn, 'p' represents a black pawn, 'K' represents a white king, and so on.
/// Determine the corresponding position on the board for each piece based on its rank and file. The rank
/// and file are represented by the row and column of the chessboard, respectively.
/// Update the bb_pieces array in the Position struct to reflect the positions of the pieces for each side.
/// Initialize the Board struct with the Position struct containing the updated piece positions.
pub fn parse_fen(fen: &str) -> miette::Result<Board> {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    let mut board = Board::default();
    if parts.len() != 6 {
        miette::bail!(
            "Not enough segments in given FEN string, need 6, got: {}",
            parts.len()
        );
    }
    let piece_placement = parts[0];
    board.positions = parse_with_context(
        piece_placement,
        place_pieces,
        "Placing pieces with given fen",
    )?;
    let stm = parts[1];
    board.stm = parse_with_context(stm, parse_stm, "Parsed stm input")?;
    let castle = parts[2];
    board.castling_rights = parse_with_context(castle, parse_castle, "Parsed castle input")?;
    let enpassant = parts[3];
    board.enpassant_square =
        parse_with_context(enpassant, parse_enpassant, "Parsed enpassant input")?;
    let half_move = parts[4];
    board.halfmove_clock = parse_with_context(
        half_move,
        |s| s.parse::<u8>().into_diagnostic(),
        "Parsed halfmove to u8",
    )?;
    let full_move = parts[5];
    board.fullmove_counter = parse_with_context(
        full_move,
        |s| s.parse::<u8>().into_diagnostic(),
        "Parsed fullmove to u8",
    )?;
    Ok(board)
}

fn parse_with_context<T, F>(input: &str, parser: F, context_msg: &str) -> miette::Result<T>
where
    F: FnOnce(&str) -> miette::Result<T>,
{
    parser(input).with_context(|| format!("{context_msg}: {input}"))
}

fn place_pieces(pieces: &str) -> miette::Result<BoardState> {
    let mut positions = BoardState::default();

    if pieces.contains(' ') {
        miette::bail!("'Space' found in 'pieces' part of FEN: {pieces}");
    }

    let mut rank = 7;
    let mut file = 0;

    for char in pieces.chars() {
        match char {
            '1'..='8' => {
                file += char
                    .to_digit(10)
                    .with_context(|| miette::miette!("Could not parse char {char} to number"))?
                    as usize;
            }
            '/' => {
                rank -= 1;
                file = 0;
            }
            _ => {
                if let Some((piece, side)) = PIECE_CHAR_LOOKUP_TABLE.get(&char) {
                    positions.set(*side, *piece, rank * 8 + file)?;
                    file += 1
                } else {
                    miette::bail!("Invalid fen character: {char}")
                }
            }
        }
    }

    Ok(positions)
}

fn parse_stm(stm: &str) -> miette::Result<Side> {
    match stm {
        "w" => Ok(Side::White),
        "b" => Ok(Side::Black),
        _ => Err(miette::Error::msg("Invalid stm")),
    }
}

fn parse_castle(castle: &str) -> miette::Result<CastlingRights> {
    let mut res = 0b0u8;
    for c in castle.chars() {
        match c {
            'K' => res |= CastlingRights::WHITE_00,
            'Q' => res |= CastlingRights::WHITE_000,
            'k' => res |= CastlingRights::BLACK_00,
            'q' => res |= CastlingRights::BLACK_000,
            '-' => res = CastlingRights::NO_CASTLING,
            _ => {
                return Err(miette::Error::msg(
                    "Unexpected character while parsing CastlingRights",
                ));
            }
        };
    }
    Ok(CastlingRights(res))
}

fn parse_enpassant(enpassant: &str) -> miette::Result<Option<Square>> {
    if enpassant == "-" {
        Ok(None)
    } else {
        let file = enpassant
            .chars()
            .next()
            .ok_or_else(|| miette::Error::msg("Missing en passant file"))?;
        let rank = enpassant
            .chars()
            .nth(1)
            .ok_or_else(|| miette::Error::msg("Missing enpassant file"))?;
        let square = Square::enpassant_from_index(file, rank)?;
        Ok(Some(square))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fen() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let board = parse_fen(fen).unwrap();
        assert_eq!(board.stm, Side::White);
        assert_eq!(board.castling_rights, CastlingRights::all());
        assert_eq!(board.enpassant_square, None);
        assert_eq!(board.halfmove_clock, 0);
        assert_eq!(board.fullmove_counter, 1);
    }

    #[test]
    fn test_parse_enpassant() {
        // Valid en passant
        let enpassant = "e3";
        let square = parse_enpassant(enpassant).unwrap().unwrap();
        assert_eq!(square, Square::new(20).unwrap()); // Adjust based on your implementation

        // Invalid en passant (missing rank)
        let enpassant_missing_rank = "e";
        assert!(parse_enpassant(enpassant_missing_rank).is_err());

        // Invalid en passant (missing file)
        let enpassant_missing_file = "";
        assert!(parse_enpassant(enpassant_missing_file).is_err());

        // En passant disabled
        let enpassant_disabled = "-";
        assert_eq!(parse_enpassant(enpassant_disabled).unwrap(), None);
    }
}
