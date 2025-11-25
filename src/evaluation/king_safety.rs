use crate::{
    evaluation::accumulator::EvalAccumulator,
    prelude::*,
    tuning::params::{
        CASTLING_BONUS, OPEN_FILE_PENALTY, PAWN_SHIELD_FULL, PAWN_SHIELD_PARTIAL,
        SEMI_OPEN_FILE_PENALTY,
    },
};

#[rustfmt::skip]
const SAFETY_TABLE: [i32; 100] = [
    0, 0, 1, 2, 3, 5, 7, 9, 12, 15,
    18, 22, 26, 30, 35, 39, 44, 50, 56, 62,
    68, 75, 82, 85, 89, 97, 105, 113, 122, 131,
    140, 150, 169, 180, 191, 202, 213, 225, 237, 248,
    260, 272, 283, 295, 307, 319, 330, 342, 354, 366,
    377, 389, 401, 412, 424, 436, 448, 459, 471, 483,
    494, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500
];

pub(crate) fn eval_king_safety(board: &Board, acc: &mut impl EvalAccumulator) {
    for side in [Side::White, Side::Black] {
        if let Some(king_sq) = board.positions.get_piece_bb(side, Piece::King).lsb() {
            eval_side_safety(board, side, king_sq as usize, acc);
        }
    }
}

fn eval_side_safety(board: &Board, side: Side, king_sq: usize, acc: &mut impl EvalAccumulator) {
    let side_idx = side.index();
    let king_file = king_sq % 8;
    let king_rank = king_sq / 8;
    // Castling Bonus
    if board.castling_rights.has_castled(side) {
        acc.add_feature(CASTLING_BONUS, side, 1);
    }

    // Pawn Shield
    let friendly_pawns = board.positions.get_piece_bb(side, Piece::Pawn);

    let shield_rank1_mask = &PAWN_TABLES.king_shield_zone_masks[side_idx][king_sq];
    let shield_rank2_mask =
        if (side == Side::White && king_rank < 6) || (side == Side::Black && king_rank > 1) {
            let square_ahead = king_sq as i32 + (if side == Side::White { 8 } else { -8 });
            PAWN_TABLES.king_shield_zone_masks[side_idx][square_ahead as usize]
        } else {
            BitBoard(0)
        };

    let total_shield_zone = *shield_rank1_mask | shield_rank2_mask;
    let shielded_pawns = friendly_pawns & &total_shield_zone;

    let mut full_shield_files = 0;
    for file_offset in [-1, 0, 1] {
        let target_file = king_file as i8 + file_offset;
        if (0..8).contains(&target_file)
            && (shielded_pawns & BitBoard(FILE_MASKS[target_file as usize])).any()
        {
            full_shield_files += 1;
        }
    }

    match full_shield_files {
        3 => acc.add_feature(PAWN_SHIELD_FULL, side, 1),
        2 => acc.add_feature(PAWN_SHIELD_PARTIAL, side, 1),
        _ => {}
    }

    // Open Files Penalty
    let opponent = side.flip();
    let opponent_pawns = board.positions.get_piece_bb(opponent, Piece::Pawn);

    for file_offset in [-1, 0, 1] {
        let target_file = king_file as i8 + file_offset;
        if (0..8).contains(&target_file) {
            let file_mask = BitBoard(FILE_MASKS[target_file as usize]);
            let has_friendly = (file_mask & *friendly_pawns).any();

            // If no friendly pawns, the file is open for 'side'
            if !has_friendly {
                let has_opponent_pawns = (file_mask & *opponent_pawns).any();
                if has_opponent_pawns {
                    // Semi-Open: No friendly pawns, but there are enemy pawns
                    acc.add_feature(SEMI_OPEN_FILE_PENALTY, side, 1);
                } else {
                    // Fully Open: No pawns of either color
                    acc.add_feature(OPEN_FILE_PENALTY, side, 1);
                }
            }
        }
    }

    // Attack Units - Fixed Score
    // Not usually touched by SPSA/Texel Tuning
    let attack_units = calculate_attack_units(board, side, king_sq);
    let safety_penalty = Score::splat(SAFETY_TABLE[attack_units.min(99) as usize]);

    // Safety table is a penalty to the side being attacked.
    // add_fixed_score(val, side) adds if White, subtracts if Black.
    // We want to SUBTRACT from White if White is unsafe.
    // So we pass -safety_penalty.
    acc.add_fixed_score(-safety_penalty, side);
}

fn calculate_attack_units(board: &Board, side: Side, king_sq: usize) -> i32 {
    let opponent = side.flip();
    let mut total_attack_units = 0;
    let king_zone_mask = PAWN_TABLES.king_attack_zone_masks[side.index()][king_sq];
    let occupied = board.positions.get_occupied_bb();

    for piece_type in Piece::all_pieces() {
        if piece_type == Piece::King {
            continue;
        }

        let mut opponent_pieces = *board.positions.get_piece_bb(opponent, piece_type);
        while let Some(from_sq) = opponent_pieces.try_pop_lsb() {
            let attacks = match piece_type {
                Piece::Pawn => MOVE_TABLES.get_pawn_attacks(from_sq as usize, opponent),
                Piece::Bishop => MOVE_TABLES.get_bishop_attacks_generic(from_sq as usize, occupied),
                Piece::Knight => MOVE_TABLES.knight_moves[from_sq as usize],
                Piece::Rook => MOVE_TABLES.get_rook_attacks_generic(from_sq as usize, occupied),
                Piece::Queen => {
                    MOVE_TABLES.get_bishop_attacks_generic(from_sq as usize, occupied)
                        | MOVE_TABLES.get_rook_attacks_generic(from_sq as usize, occupied)
                }
                _ => BitBoard(0),
            };

            if (attacks & king_zone_mask).any() {
                total_attack_units += match piece_type {
                    Piece::Pawn => 1, // Usually pawns don't count for "attack units" in this specific heuristic
                    Piece::Knight => 2,
                    Piece::Bishop => 2,
                    Piece::Rook => 3,
                    Piece::Queen => 5,
                    _ => 0,
                };
            }
        }
    }
    total_attack_units
}
