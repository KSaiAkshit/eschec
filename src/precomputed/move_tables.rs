use crate::{moves::magics::MagicEntry, prelude::*};

#[derive(Debug)]
pub struct MoveTables {
    // Basic movement patterns
    pub knight_moves: [BitBoard; 64],
    pub king_moves: [BitBoard; 64],
    pub white_pawn_attacks: [BitBoard; 64],
    pub black_pawn_attacks: [BitBoard; 64],

    pub white_pawn_pushes: [BitBoard; 64],
    pub black_pawn_pushes: [BitBoard; 64],
    // Double pushes from start rank
    pub white_pawn_double_pushes: [BitBoard; 64],
    // Double pushes from start rank
    pub black_pawn_double_pushes: [BitBoard; 64],

    pub rook_magics: [MagicEntry; 64],
    pub bishop_magics: [MagicEntry; 64],

    // For sliding pieces, pre-compute ray attacks
    // These represent attacks in each direction, assuming empty board
    pub north_rays: [BitBoard; 64],
    pub south_rays: [BitBoard; 64],
    pub east_rays: [BitBoard; 64],
    pub west_rays: [BitBoard; 64],
    pub northeast_rays: [BitBoard; 64],
    pub southeast_rays: [BitBoard; 64],
    pub southwest_rays: [BitBoard; 64],
    pub northwest_rays: [BitBoard; 64],
}

// pub static MOVE_TABLES: LazyLock<MoveTables> = LazyLock::new(MoveTables::new);
pub const MOVE_TABLES: MoveTables = MoveTables::new();

impl Default for MoveTables {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveTables {
    pub const fn new() -> Self {
        let mut tables = Self {
            knight_moves: [BitBoard(0); 64],
            king_moves: [BitBoard(0); 64],
            white_pawn_attacks: [BitBoard(0); 64],
            black_pawn_attacks: [BitBoard(0); 64],
            white_pawn_pushes: [BitBoard(0); 64],
            black_pawn_pushes: [BitBoard(0); 64],
            white_pawn_double_pushes: [BitBoard(0); 64],
            black_pawn_double_pushes: [BitBoard(0); 64],
            rook_magics: [MagicEntry::EMPTY_MAGIC; 64],
            bishop_magics: [MagicEntry::EMPTY_MAGIC; 64],
            north_rays: [BitBoard(0); 64],
            south_rays: [BitBoard(0); 64],
            east_rays: [BitBoard(0); 64],
            west_rays: [BitBoard(0); 64],
            northeast_rays: [BitBoard(0); 64],
            southeast_rays: [BitBoard(0); 64],
            southwest_rays: [BitBoard(0); 64],
            northwest_rays: [BitBoard(0); 64],
        };

        tables.init_knight_moves();
        tables.init_king_moves();
        tables.init_pawn_tables();
        tables.init_magics();
        tables.init_ray_attacks();

        tables
    }

    const fn init_knight_moves(&mut self) {
        let knight_offsets: [i8; 8] = [-17, -15, -10, -6, 6, 10, 15, 17];

        let mut index = 0;
        while index < 64 {
            let rank = index / 8;
            let file = index % 8;
            let mut knight_moves = BitBoard(0);

            let mut i = 0;
            while i < 8 {
                let offset = knight_offsets[i];
                let target = index as i8 + offset;
                if target >= 0 && target < 64 {
                    let target_rank = target as usize / 8;
                    let target_file = target as usize % 8;

                    let rank_diff = (rank as i8 - target_rank as i8).abs();
                    let file_diff = (file as i8 - target_file as i8).abs();

                    if (rank_diff == 2 && file_diff == 1) || (rank_diff == 1 && file_diff == 2) {
                        knight_moves.set(target as usize);
                    }
                }
                i += 1;
            }
            self.knight_moves[index] = knight_moves;
            index += 1;
        }
    }

    const fn init_king_moves(&mut self) {
        // Directions: horizontal, vertical, diagonal
        let king_offsets: [i8; 8] = [-9, -8, -7, -1, 1, 7, 8, 9];

        let mut index = 0;
        while index < 64 {
            let rank = index / 8;
            let file = index % 8;
            let mut king_moves = BitBoard(0);

            let mut i = 0;
            while i < 8 {
                let offset = king_offsets[i];
                let target_index = index as i8 + offset;
                if target_index >= 0 && target_index < 64 {
                    let target_rank = target_index as usize / 8;
                    let target_file = target_index as usize % 8;

                    let rank_diff = (rank as i8 - target_rank as i8).abs();
                    let file_diff = (file as i8 - target_file as i8).abs();

                    if rank_diff <= 1 && file_diff <= 1 {
                        king_moves.set(target_index as usize);
                    }
                }
                i += 1;
            }
            self.king_moves[index] = king_moves;
            index += 1;
        }
    }

    const fn init_magics(&mut self) {
        let mut i = 0;
        while i < 64 {
            self.rook_magics[i] = MagicEntry {
                mask: BitBoard(magics::ROOK_MASKS[i]),
                magic: magics::ROOK_MAGICS[i],
                offset: magics::ROOK_ATTACK_OFFSETS[i],
                shift: magics::ROOK_SHIFTS[i] as u32,
            };
            self.bishop_magics[i] = MagicEntry {
                mask: BitBoard(magics::BISHOP_MASKS[i]),
                magic: magics::BISHOP_MAGICS[i],
                offset: magics::BISHOP_ATTACK_OFFSETS[i],
                shift: magics::BISHOP_SHIFTS[i] as u32,
            };
            i += 1;
        }
    }

    const fn generate_ray(
        &self,
        start_rank: usize,
        start_file: usize,
        direction: Direction,
    ) -> BitBoard {
        let mut ray = BitBoard(0);

        let (dr, df) = direction.deltas();

        let mut rank = start_rank as i8;
        let mut file = start_file as i8;

        loop {
            rank += dr;
            file += df;

            if rank < 0 || rank >= 8 || file < 0 || file >= 8 {
                break;
            }

            let idx = (rank as usize) * 8 + (file as usize);
            ray.set(idx);
        }

        ray
    }

    const fn init_pawn_tables(&mut self) {
        let mut index = 0;
        while index < 64 {
            let rank = index / 8;
            let file = index % 8;

            // White pawn attacks
            let mut white_attacks = BitBoard(0);
            if rank < 7 {
                if file > 0 {
                    white_attacks.set((rank + 1) * 8 + file - 1);
                }
                if file < 7 {
                    white_attacks.set((rank + 1) * 8 + file + 1);
                }
            }
            self.white_pawn_attacks[index] = white_attacks;

            // Black pawn attacks
            let mut black_attacks = BitBoard(0);
            if rank > 0 {
                if file > 0 {
                    black_attacks.set((rank - 1) * 8 + file - 1);
                }
                if file < 7 {
                    black_attacks.set((rank - 1) * 8 + file + 1);
                }
            }
            self.black_pawn_attacks[index] = black_attacks;

            // White pawn single pushes
            let mut white_pushes = BitBoard(0);
            if rank < 7 {
                white_pushes.set((rank + 1) * 8 + file);
            }
            self.white_pawn_pushes[index] = white_pushes;

            // Black pawn single pushes
            let mut black_pushes = BitBoard(0);
            if rank > 0 {
                black_pushes.set((rank - 1) * 8 + file);
            }
            self.black_pawn_pushes[index] = black_pushes;

            // White pawn single pushes
            let mut white_double_pushes = BitBoard(0);
            if rank == 1 {
                white_double_pushes.set((rank + 2) * 8 + file);
            }
            self.white_pawn_double_pushes[index] = white_double_pushes;

            // Black pawn single pushes
            let mut black_double_pushes = BitBoard(0);
            if rank == 6 {
                black_double_pushes.set((rank - 2) * 8 + file);
            }
            self.black_pawn_double_pushes[index] = black_double_pushes;
            index += 1;
        }
    }

    const fn init_ray_attacks(&mut self) {
        let mut index = 0;
        while index < 64 {
            let rank = index / 8;
            let file = index % 8;

            self.north_rays[index] = self.generate_ray(rank, file, Direction::NORTH);
            self.south_rays[index] = self.generate_ray(rank, file, Direction::SOUTH);
            self.east_rays[index] = self.generate_ray(rank, file, Direction::EAST);
            self.west_rays[index] = self.generate_ray(rank, file, Direction::WEST);
            self.northeast_rays[index] = self.generate_ray(rank, file, Direction::NORTHEAST);
            self.southeast_rays[index] = self.generate_ray(rank, file, Direction::SOUTHEAST);
            self.southwest_rays[index] = self.generate_ray(rank, file, Direction::SOUTHWEST);
            self.northwest_rays[index] = self.generate_ray(rank, file, Direction::NORTHWEST);

            index += 1
        }
    }

    pub fn generate_sliding_attack_mask(&self, from: usize, is_rook: bool) -> BitBoard {
        let mut attacks = BitBoard(0);
        let all_rays = if is_rook {
            [
                self.north_rays[from],
                self.south_rays[from],
                self.east_rays[from],
                self.west_rays[from],
            ]
        } else {
            [
                self.northeast_rays[from],
                self.southeast_rays[from],
                self.southwest_rays[from],
                self.northwest_rays[from],
            ]
        };

        for ray in all_rays {
            if ray.is_empty() {
                continue;
            }
            // Get the square at the far end of the ray
            let edge_sq = if ray.0 > (1u64 << from) {
                ray.msb().unwrap() as usize
            } else {
                ray.lsb().unwrap() as usize
            };

            // Get the ray coming back from that edge square
            let reverse_ray = if is_rook {
                self.get_rook_moves(edge_sq, BitBoard(0), BitBoard(0))
            } else {
                self.get_bishop_moves(edge_sq, BitBoard(0), BitBoard(0))
            };

            // The intersection of the forward and reverse rays gives us the squares *between*
            // the piece and the edge, which is exactly what we need for the mask.
            attacks |= ray & reverse_ray;
        }

        attacks
    }

    pub const fn get_rook_moves(
        &self,
        from: usize,
        ally_pieces: BitBoard,
        enemy_pieces: BitBoard,
    ) -> BitBoard {
        let blockers = ally_pieces.or(enemy_pieces);
        let attacks = self.get_rook_attacks_bb(from, blockers);
        attacks.and(ally_pieces.not())
    }

    pub const fn get_bishop_moves(
        &self,
        from: usize,
        ally_pieces: BitBoard,
        enemy_pieces: BitBoard,
    ) -> BitBoard {
        let blockers = ally_pieces.or(enemy_pieces);
        let attacks = self.get_bishop_attacks_bb(from, blockers);
        attacks.and(ally_pieces.not())
    }

    pub const fn get_queen_moves(
        &self,
        from: usize,
        ally_pieces: BitBoard,
        enemy_pieces: BitBoard,
    ) -> BitBoard {
        self.get_rook_moves(from, ally_pieces, enemy_pieces)
            .or(self.get_bishop_moves(from, ally_pieces, enemy_pieces))
    }

    pub const fn ray_until_blocker(
        &self,
        ray: BitBoard,
        ally_pieces: BitBoard,
        enemy_pieces: BitBoard,
        forward: bool,
    ) -> BitBoard {
        let blockers = ray.and(ally_pieces.or(enemy_pieces));

        // No blockers, return the whole ray
        if blockers.0 == 0 {
            return ray;
        }

        let maybe_blocker = if forward {
            blockers.lsb()
        } else {
            blockers.msb()
        };
        if let Some(index) = maybe_blocker {
            let blocker_mask = 1u64 << index;
            let mask_up_to_blocker = if forward {
                blocker_mask | (blocker_mask - 1)
            } else {
                !(blocker_mask - 1)
            };
            let ray_up_to_blocker = ray.0 & mask_up_to_blocker;
            if (blocker_mask & ally_pieces.0) != 0 {
                BitBoard(ray_up_to_blocker & !blocker_mask)
            } else {
                BitBoard(ray_up_to_blocker)
            }
        } else {
            ray
        }
    }

    pub fn get_pawn_attacks(&self, from: usize, side: Side) -> BitBoard {
        match side {
            Side::White => self.white_pawn_attacks[from],
            Side::Black => self.black_pawn_attacks[from],
        }
    }

    pub fn get_pawn_pushes(
        &self,
        from: usize,
        side: Side,
        ally_pieces: BitBoard,
        enemy_pieces: BitBoard,
    ) -> BitBoard {
        let occupied = ally_pieces | enemy_pieces;
        let mut moves = BitBoard(0);

        match side {
            Side::White => {
                let single_push = self.white_pawn_pushes[from];
                if (single_push & occupied).0 == 0 {
                    moves |= single_push;

                    let double_push = self.white_pawn_double_pushes[from];
                    if double_push.0 != 0 && (double_push & occupied).0 == 0 {
                        moves |= double_push;
                    }
                }
            }
            Side::Black => {
                let single_push = self.black_pawn_pushes[from];
                if (single_push & occupied).0 == 0 {
                    moves |= single_push;

                    let double_push = self.black_pawn_double_pushes[from];
                    if double_push.0 != 0 && (double_push & occupied).0 == 0 {
                        moves |= double_push;
                    }
                }
            }
        }

        moves
    }

    pub fn get_ray(&self, from: usize, dir: Direction) -> BitBoard {
        match dir {
            Direction::NORTH => self.north_rays[from],
            Direction::SOUTH => self.south_rays[from],
            Direction::EAST => self.east_rays[from],
            Direction::WEST => self.west_rays[from],
            Direction::NORTHEAST => self.northeast_rays[from],
            Direction::SOUTHEAST => self.southeast_rays[from],
            Direction::SOUTHWEST => self.southwest_rays[from],
            Direction::NORTHWEST => self.northwest_rays[from],
            _ => BitBoard(0),
        }
    }

    // "generic" functions that do not care about ally/enemy
    // They take in an 'occupied' bitboard that contain all pieces except the king
    // This is important for calculating opponent's attack map
    //

    fn get_attacks_in_dir(
        &self,
        ray: BitBoard,
        occupied: BitBoard,
        is_positive_direction: bool,
        blocker_ray_table: &[BitBoard; 64],
    ) -> BitBoard {
        let blockers = ray & occupied;

        if let Some(first_blocker_idx) = blockers.get_closest_bit(is_positive_direction) {
            ray & !blocker_ray_table[first_blocker_idx as usize]
        } else {
            ray
        }
    }

    pub fn get_bishop_attacks_generic(&self, from: usize, occupied: BitBoard) -> BitBoard {
        let ne_attacks = self.get_attacks_in_dir(
            self.northeast_rays[from],
            occupied,
            true,
            &self.northeast_rays,
        );
        let se_attacks = self.get_attacks_in_dir(
            self.southeast_rays[from],
            occupied,
            false,
            &self.southeast_rays,
        );
        let sw_attacks = self.get_attacks_in_dir(
            self.southwest_rays[from],
            occupied,
            false,
            &self.southwest_rays,
        );
        let nw_attacks = self.get_attacks_in_dir(
            self.northwest_rays[from],
            occupied,
            true,
            &self.northwest_rays,
        );

        ne_attacks | se_attacks | sw_attacks | nw_attacks
    }

    pub fn get_rook_attacks_generic(&self, from: usize, occupied: BitBoard) -> BitBoard {
        let n_attacks =
            self.get_attacks_in_dir(self.north_rays[from], occupied, true, &self.north_rays);
        let s_attacks =
            self.get_attacks_in_dir(self.south_rays[from], occupied, false, &self.south_rays);
        let e_attacks =
            self.get_attacks_in_dir(self.east_rays[from], occupied, true, &self.east_rays);
        let w_attacks =
            self.get_attacks_in_dir(self.west_rays[from], occupied, false, &self.west_rays);

        n_attacks | s_attacks | e_attacks | w_attacks
    }

    const fn get_rook_attacks_bb(&self, from: usize, blockers: BitBoard) -> BitBoard {
        let entry = &self.rook_magics[from];
        let blockers_masked = blockers.and(entry.mask);
        let index = ((blockers_masked.0.wrapping_mul(entry.magic)) >> entry.shift) as usize;

        magics::ROOK_ATTACKS[magics::ROOK_ATTACK_OFFSETS[from] + index]
    }

    const fn get_bishop_attacks_bb(&self, square: usize, blockers: BitBoard) -> BitBoard {
        let entry = &self.bishop_magics[square];
        let blockers_masked = blockers.and(entry.mask);
        let index = ((blockers_masked.0.wrapping_mul(entry.magic)) >> entry.shift) as usize;

        // Use indexing which is bounds-checked, instead of pointer arithmetic
        magics::BISHOP_ATTACKS[magics::BISHOP_ATTACK_OFFSETS[square] + index]
    }
}
