use crate::prelude::*;

#[derive(Debug)]
pub struct PawnTables {
    /// For isolated pawn check: combines FILE_MASKS for adjacent files
    ///
    /// *Index: \[file\] -> BitBoard ( file-1 | file+1)*
    pub pawn_adjacent_files_masks: [BitBoard; 8],

    /// For passed pawn check: contains adjacent files and all sq infront of 'sq'
    ///
    /// *Index: \[Side\]\[square\] -> BitBoard*
    pub passed_pawn_blocking_masks: [[BitBoard; 64]; 2],

    /// For king shield: pawn shield relative to king's square
    ///
    /// *Index: \[Side\]\[king_square\] -> BitBoard ( 3 squares on rank in front of the king )*
    pub king_shield_zone_masks: [[BitBoard; 64]; 2],

    /// For backward pawn check: square directly in front of the pawn
    ///
    /// *Index: \[Side\]\[pawn_square\] -> BitBoard of 1 square*
    pub pawn_front_square_masks: [[BitBoard; 64]; 2],

    /// For backward pawn check: squares directly behind and to the sides (potential support).
    ///
    /// *Index: \[Side\]\[pawn_square\] -> BitBoard of 2 squares*
    pub pawn_backward_support_masks: [[BitBoard; 64]; 2],

    /// For connected pawns: contains twon squares on adjacent files on the same rank
    ///
    /// *Index: \[pawn_square\] -> BitBoard of 2 neighbours*
    pub connected_pawn_masks: [BitBoard; 64],

    /// For King Safety evaluation: the full 4x3 (or smaller near edges)
    /// zone of squares around the king, including two ranks "in front".
    /// This is the area enemy pieces attack.
    /// Ranks: king_rank [-1, 0, 1, 2]
    /// Files: king_file [-1, 0, 1]
    ///
    /// *Index: \[Side\]\[king_square\] -> BitBoard*
    pub king_attack_zone_masks: [[BitBoard; 64]; 2],
}

pub const PAWN_TABLES: PawnTables = PawnTables::new();

impl Default for PawnTables {
    fn default() -> Self {
        Self {
            pawn_adjacent_files_masks: [BitBoard(0); 8],
            passed_pawn_blocking_masks: [[BitBoard(0); 64]; 2],
            king_shield_zone_masks: [[BitBoard(0); 64]; 2],
            pawn_front_square_masks: [[BitBoard(0); 64]; 2],
            pawn_backward_support_masks: [[BitBoard(0); 64]; 2],
            connected_pawn_masks: [BitBoard(0); 64],
            king_attack_zone_masks: [[BitBoard(0); 64]; 2],
        }
    }
}

impl PawnTables {
    pub const fn new() -> Self {
        let mut tables = Self {
            pawn_adjacent_files_masks: [BitBoard(0); 8],
            passed_pawn_blocking_masks: [[BitBoard(0); 64]; 2],
            king_shield_zone_masks: [[BitBoard(0); 64]; 2],
            pawn_front_square_masks: [[BitBoard(0); 64]; 2],
            pawn_backward_support_masks: [[BitBoard(0); 64]; 2],
            connected_pawn_masks: [BitBoard(0); 64],
            king_attack_zone_masks: [[BitBoard(0); 64]; 2],
        };
        tables.init_adjacent_file_masks();
        tables.init_per_sq_masks();
        tables.init_king_attack_zones();
        tables
    }

    const fn init_adjacent_file_masks(&mut self) {
        let mut file = 0;
        while file < 8 {
            let mut adj_files_mask = BitBoard(0);
            if file > 0 {
                adj_files_mask.0 |= FILE_MASKS[file - 1];
            }
            if file < 7 {
                adj_files_mask.0 |= FILE_MASKS[file + 1];
            }
            self.pawn_adjacent_files_masks[file] = adj_files_mask;
            file += 1;
        }
    }

    const fn init_per_sq_masks(&mut self) {
        let mut sq_idx = 0;
        while sq_idx < 64 {
            let rank = sq_idx / 8;
            let file = sq_idx % 8;

            // passed_pawn_blocking_masks: Squares in front of a pawn on its file and adjacent files.
            let mut white_passed_mask = BitBoard(0);
            let mut r = rank + 1;
            while r < 8 {
                let mut file_delta = -1;
                while file_delta <= 1 {
                    if (file as i8 + file_delta) >= 0 && (file as i8 + file_delta) < 8 {
                        white_passed_mask.set(r * 8 + (file as i8 + file_delta) as usize);
                    }
                    file_delta += 1;
                }
                r += 1;
            }
            self.passed_pawn_blocking_masks[Side::White.index()][sq_idx] = white_passed_mask;
            let mut black_passed_mask = BitBoard(0);
            let mut r = rank as i8 - 1;
            while r >= 0 {
                let mut file_delta = -1;
                while file_delta <= 1 {
                    if (file as i8 + file_delta) >= 0 && (file as i8 + file_delta) < 8 {
                        black_passed_mask.set(r as usize * 8 + (file as i8 + file_delta) as usize);
                    }
                    file_delta += 1;
                }
                r -= 1;
            }
            self.passed_pawn_blocking_masks[Side::Black.index()][sq_idx] = black_passed_mask;

            // king_shield_zone_masks: 3 squares in front of the king (relative to pawn advance).
            let mut white_king_shield_mask = BitBoard(0);
            if rank < 7 {
                let shield_rank = rank + 1;
                let mut file_delta = -1;
                while file_delta <= 1 {
                    if (file as i8 + file_delta) >= 0 && (file as i8 + file_delta) < 8 {
                        white_king_shield_mask
                            .set(shield_rank * 8 + (file as i8 + file_delta) as usize);
                    }
                    file_delta += 1;
                }
            }
            self.king_shield_zone_masks[Side::White.index()][sq_idx] = white_king_shield_mask;
            let mut black_king_shield_mask = BitBoard(0);
            if rank > 0 {
                let shield_rank = rank - 1;
                let mut file_delta = -1;
                while file_delta <= 1 {
                    if (file as i8 + file_delta) >= 0 && (file as i8 + file_delta) < 8 {
                        black_king_shield_mask
                            .set(shield_rank * 8 + (file as i8 + file_delta) as usize);
                    }
                    file_delta += 1;
                }
            }
            self.king_shield_zone_masks[Side::Black.index()][sq_idx] = black_king_shield_mask;

            // pawn_front_square_masks: The square directly in front of the pawn.
            let mut north_sq_mask = BitBoard(0);
            if rank < 7 {
                north_sq_mask.set((rank + 1) * 8 + file);
            }
            self.pawn_front_square_masks[Side::White.index()][sq_idx] = north_sq_mask;
            let mut south_sq_mask = BitBoard(0);
            if rank > 0 {
                south_sq_mask.set((rank - 1) * 8 + file);
            }
            self.pawn_front_square_masks[Side::Black.index()][sq_idx] = south_sq_mask;

            // pawn_backward_support_masks: Squares directly behind and to the sides (potential support).
            let mut north_support_mask = BitBoard(0);
            if rank > 0 {
                if file > 0 {
                    north_support_mask.set((rank - 1) * 8 + file - 1);
                }
                if file < 7 {
                    north_support_mask.set((rank - 1) * 8 + file + 1);
                }
            }
            self.pawn_backward_support_masks[Side::White.index()][sq_idx] = north_support_mask;
            let mut south_support_mask = BitBoard(0);
            if rank < 7 {
                if file > 0 {
                    south_support_mask.set((rank + 1) * 8 + file - 1);
                }
                if file < 7 {
                    south_support_mask.set((rank + 1) * 8 + file + 1);
                }
            }
            self.pawn_backward_support_masks[Side::Black.index()][sq_idx] = south_support_mask;

            let mut neighbour_mask = BitBoard(0);
            if file > 0 {
                neighbour_mask.set(rank * 8 + (file - 1));
            }
            if file < 7 {
                neighbour_mask.set(rank * 8 + (file + 1));
            }
            self.connected_pawn_masks[sq_idx] = neighbour_mask;
            sq_idx += 1;
        }
    }

    const fn init_king_attack_zones(&mut self) {
        let mut sq_idx: usize = 0;
        while sq_idx < 64 {
            let king_rank = sq_idx / 8;
            let king_file = sq_idx % 8;

            let mut white_zone = BitBoard(0);
            let mut rank_delta: i8 = -1;
            while rank_delta <= 2 {
                let current_rank_i8 = king_rank as i8 + rank_delta;
                if current_rank_i8 >= 0 && current_rank_i8 < 8 {
                    let current_rank = current_rank_i8 as usize;

                    let mut f_delta = -1;
                    while f_delta <= 1 {
                        let current_file_i8 = king_file as i8 + f_delta;
                        if current_file_i8 >= 0 && current_file_i8 < 8 {
                            white_zone.set(current_rank * 8 + current_file_i8 as usize);
                        }
                        f_delta += 1;
                    }
                }
                rank_delta += 1;
            }
            white_zone.capture(sq_idx);
            self.king_attack_zone_masks[Side::White.index()][sq_idx] = white_zone;

            let mut black_zone = BitBoard(0);
            let mut r_delta: i8 = -1;
            while r_delta <= 2 {
                let current_rank_i8 = king_rank as i8 - r_delta;
                if current_rank_i8 >= 0 && current_rank_i8 < 8 {
                    let current_rank = current_rank_i8 as usize;

                    let mut f_delta = -1;
                    while f_delta <= 1 {
                        let current_file_i8 = king_file as i8 + f_delta;
                        if current_file_i8 >= 0 && current_file_i8 < 8 {
                            black_zone.set(current_rank * 8 + current_file_i8 as usize);
                        }
                        f_delta += 1;
                    }
                }
                r_delta += 1;
            }
            black_zone.capture(sq_idx);
            self.king_attack_zone_masks[Side::Black.index()][sq_idx] = black_zone;

            sq_idx += 1;
        }
    }
}
