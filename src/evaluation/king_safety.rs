use crate::{
    evaluation::accumulator::EvalAccumulator,
    prelude::*,
    tuning::params::{
        CASTLING_BONUS, OPEN_FILE_PENALTY, PAWN_SHIELD_FULL, PAWN_SHIELD_PARTIAL,
        POTENTIAL_OPEN_FILE_PENALTY, SEMI_OPEN_FILE_PENALTY,
    },
};

// Max index for the safety table
const SAFETY_MAX: usize = 100;

const NO_QUEEN_BONUS: u32 = 8;

// The "Exploding" Table
// Maps linear attack units to exponential danger scores (in centipawns).
// 0-9 units = negligible danger
// 20+ units = serious danger
#[rustfmt::skip]
const SAFETY_TABLE: [i32; SAFETY_MAX + 1] = [
    0, 0, 0, 0, 0, 1, 1, 2, 2, 3,           // 0-9
    4, 5, 6, 8, 10, 12, 14, 17, 20, 23,     // 10-19
    27, 31, 36, 41, 46, 52, 58, 65, 72, 80, // 20-29
    88, 97, 106, 116, 126, 137, 148, 160, 172, 185, // 30-39
    199, 213, 228, 243, 259, 276, 293, 311, 330, 349, // 40-49
    369, 390, 411, 433, 456, 479, 503, 528, 554, 580, // 50-59
    607, 635, 664, 693, 723, 754, 786, 819, 852, 886, // 60-69
    921, 957, 994, 1032, 1070, 1110, 1150, 1191, 1233, 1276, // 70-79
    1320, 1365, 1411, 1458, 1506, 1555, 1605, 1656, 1708, 1761, // 80-89
    1815, 1870, 1926, 1983, 2041, 2100, 2160, 2221, 2283, 2346, // 90-99
    2410 // 100
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
    let enemy_majors = board.positions.get_ortho_sliders_bb(opponent);
    let has_enemy_majors = enemy_majors.any();

    for file_offset in [-1, 0, 1] {
        let target_file = king_file as i8 + file_offset;
        if !(0..8).contains(&target_file) {
            continue;
        }

        let file_mask = BitBoard(FILE_MASKS[target_file as usize]);

        // Friendly pawn blocks the file - all good
        if (file_mask & *friendly_pawns).any() {
            continue;
        }

        // No enemy majors at all - no file dangers
        if !has_enemy_majors {
            continue;
        }
        let major_on_file = (file_mask & enemy_majors).any();
        let has_opponent_pawns = (file_mask & *opponent_pawns).any();

        if major_on_file {
            if has_opponent_pawns {
                acc.add_feature(SEMI_OPEN_FILE_PENALTY, side, 1);
            } else {
                acc.add_feature(OPEN_FILE_PENALTY, side, 1);
            }
        } else {
            acc.add_feature(POTENTIAL_OPEN_FILE_PENALTY, side, 1);
        }
    }

    // Attack Units - Fixed Score
    // Not usually touched by SPSA/Texel Tuning
    let mut attack_units = calculate_attack_units(board, side, king_sq);
    if attack_units > 2 {
        let has_enemy_queen = !board
            .positions
            .get_piece_bb(opponent, Piece::Queen)
            .is_empty();

        // If the Queen is gone, the attack needs to be significantly heavier (more pieces)
        // to generate the same danger score.
        if !has_enemy_queen {
            // Subtracting 8 units is roughly equivalent to removing a "Queen + Bishop" worth of pressure.
            // This means Rooks need a lot of help to generate danger.
            attack_units = attack_units.saturating_sub(NO_QUEEN_BONUS);
        }

        let idx: usize = attack_units.min(SAFETY_MAX as u32) as usize;

        let mg_penalty = SAFETY_TABLE[idx];

        // Safety table is a penalty to the side being attacked.
        // add_fixed_score(val, side) adds if White, subtracts if Black.
        // We want to SUBTRACT from White if White is unsafe.
        // So we pass -safety_penalty.
        if mg_penalty > 0 {
            let safety_score = Score::new(mg_penalty, 0);
            acc.add_fixed_score(-safety_score, side);
        }
    }
}

fn calculate_attack_units(board: &Board, side: Side, king_sq: usize) -> u32 {
    let opponent = side.flip();
    let king_zone_mask = PAWN_TABLES.king_attack_zone_masks[side.index()][king_sq];
    let occupied = board.positions.get_occupied_bb();

    let mut total_attack_units = 0;
    let mut queen_hits = false;
    let mut rook_hits = false;

    for piece_type in Piece::all_pieces() {
        if piece_type == Piece::King || piece_type == Piece::Pawn {
            continue;
        }

        let mut opponent_pieces = *board.positions.get_piece_bb(opponent, piece_type);
        while let Some(from_sq) = opponent_pieces.try_pop_lsb() {
            let attacks = match piece_type {
                Piece::Bishop => MOVE_TABLES.get_bishop_attacks_bb(from_sq as usize, occupied),
                Piece::Knight => MOVE_TABLES.knight_moves[from_sq as usize],
                Piece::Rook => MOVE_TABLES.get_rook_attacks_bb(from_sq as usize, occupied),
                Piece::Queen => {
                    MOVE_TABLES.get_bishop_attacks_bb(from_sq as usize, occupied)
                        | MOVE_TABLES.get_rook_attacks_bb(from_sq as usize, occupied)
                }
                _ => BitBoard(0),
            };

            let hits = attacks & king_zone_mask;
            if hits.is_empty() {
                continue;
            }

            let units = match piece_type {
                Piece::Bishop => 2,
                Piece::Knight => 2,
                Piece::Rook => 3,
                Piece::Queen => 5,
                _ => 0,
            };

            total_attack_units += units;

            if piece_type == Piece::Queen {
                queen_hits = true;
            } else if piece_type == Piece::Rook {
                rook_hits = true;
            }
        }
    }
    // Coordination bonus
    if queen_hits && rook_hits {
        total_attack_units += 2;
    }
    total_attack_units
}
