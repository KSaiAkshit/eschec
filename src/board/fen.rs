use anyhow::Context;

use super::{
    components::{CastlingRights, Side, Square},
    Board,
};

/// Parse the FEN string to extract the piece placement part. This part of the FEN string represents the positions of the pieces on the board.
/// Iterate over each character of the piece placement part of the FEN string.
/// Map each character to the corresponding piece type and side using a lookup table. For example,
/// 'P' represents a white pawn, 'p' represents a black pawn, 'K' represents a white king, and so on.
/// Determine the corresponding position on the board for each piece based on its rank and file. The rank
/// and file are represented by the row and column of the chessboard, respectively.
/// Update the bb_pieces array in the Position struct to reflect the positions of the pieces for each side.
/// You'll need to update the appropriate BitBoard for each piece type and side.
/// Ensure that the bb_sides array in the Position struct is updated accordingly to reflect the presence of pieces on each side of the board.
/// Initialize the Board struct with the Position struct containing the updated piece positions.
pub fn parse_fen(fen: &str) -> anyhow::Result<Board> {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    let mut board = Board::default();
    if parts.len() != 6 {
        return Err(anyhow::Error::msg(
            "Not enough segments in given FEN string",
        ));
    }
    let piece_placement = parts[0];
    board
        .place_pieces(piece_placement)
        .with_context(|| format!("Placing peices with given fen string {}", piece_placement))?;
    let stm = parts[1];
    board.stm = parse_stm(stm).with_context(|| format!("parsed stm input: {}", stm))?;
    let castle = parts[2];
    board.castling_rights =
        parse_castle(castle).with_context(|| format!("parsed input castle: {}", castle))?;
    let enpassant = parts[3];
    board.enpassant_square = parse_enpassant(enpassant)
        .with_context(|| format!("parsed input enpassant: {}", enpassant))?;
    let half_move = parts[4];
    board.halfmove_clock = half_move
        .parse::<u8>()
        .with_context(|| format!("attempt to parse {} to u8", half_move))?;
    let full_move = parts[5];
    board.fullmove_counter = full_move
        .parse::<u8>()
        .with_context(|| format!("attempt to parse {} to u8", full_move))?;
    Ok(board)
}

fn parse_stm(stm: &str) -> anyhow::Result<Side> {
    match stm {
        "w" => Ok(Side::White),
        "b" => Ok(Side::Black),
        _ => Err(anyhow::Error::msg("Invalid stm")),
    }
}

fn parse_castle(castle: &str) -> anyhow::Result<CastlingRights> {
    let mut res = 0b0u8;
    for c in castle.chars() {
        match c {
            'K' => res |= CastlingRights::WHITE_00,
            'Q' => res |= CastlingRights::WHITE_000,
            'k' => res |= CastlingRights::BLACK_00,
            'q' => res |= CastlingRights::BLACK_000,
            '-' => res = CastlingRights::NO_CASTLING,
            _ => {
                return Err(anyhow::Error::msg(
                    "Unexpected character while parsing CastlingRights",
                ))
            }
        };
    }
    Ok(CastlingRights(res))
}

fn parse_enpassant(enpassant: &str) -> anyhow::Result<Option<Square>> {
    if enpassant == "-" {
        Ok(None)
    } else {
        let file = enpassant
            .chars()
            .next()
            .ok_or_else(|| anyhow::Error::msg("Missing en passant file"))?;
        let rank = enpassant
            .chars()
            .nth(1)
            .ok_or_else(|| anyhow::Error::msg("Missing enpassant file"))?;
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
