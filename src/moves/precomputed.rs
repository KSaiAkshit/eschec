use crate::{BitBoard, Side, moves::Direction};

#[derive(Debug)]
pub struct MoveTables {
    // Basic movement patterns
    pub knight_moves: [BitBoard; 64],
    pub king_moves: [BitBoard; 64],
    pub white_pawn_attacks: [BitBoard; 64],
    pub black_pawn_attacks: [BitBoard; 64],

    pub white_pawn_pushes: [BitBoard; 64],
    pub black_pawn_pushes: [BitBoard; 64],
    pub white_pawn_double_pushes: [BitBoard; 64], // Double pushes from start rank
    pub black_pawn_double_pushes: [BitBoard; 64], // Double pushes from start rank

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

    const fn generate_ray(&self, start_rank: usize, start_file: usize, direction: i8) -> BitBoard {
        let mut ray = BitBoard(0);
        let start_index = start_rank * 8 + start_file;
        let mut current = start_index as i8 + direction;

        while current >= 0 && current < 64 {
            let current_rank = (current as usize) / 8;
            let current_file = (current as usize) % 8;

            let rank_diff = (current_rank as i8) - (start_rank as i8);
            let file_diff = (current_file as i8) - (start_file as i8);

            match direction {
                Direction::NORTH => {
                    if rank_diff <= 0 {
                        break;
                    }
                }
                Direction::SOUTH => {
                    if rank_diff >= 0 {
                        break;
                    }
                }
                Direction::EAST => {
                    if file_diff <= 0 || rank_diff != 0 {
                        break;
                    }
                }
                Direction::WEST => {
                    if file_diff >= 0 || rank_diff != 0 {
                        break;
                    }
                }
                Direction::NORTHEAST => {
                    if rank_diff != file_diff || rank_diff <= 0 {
                        break;
                    }
                }
                Direction::SOUTHWEST => {
                    if rank_diff != file_diff || rank_diff >= 0 {
                        break;
                    }
                }
                Direction::SOUTHEAST => {
                    if rank_diff != -file_diff || rank_diff >= 0 {
                        break;
                    }
                }
                Direction::NORTHWEST => {
                    if rank_diff != -file_diff || rank_diff <= 0 {
                        break;
                    }
                }
                _ => break,
            }
            ray.set(current as usize);

            current += direction;
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

    // const fn init_ray_attacks(&mut self) {
    //     let mut index = 0;
    //     while index < 64 {
    //         let rank = index / 8;
    //         let file = index % 8;
    //
    //         // North ray (up)
    //         let mut north = BitBoard(0);
    //         let mut r = rank + 1;
    //         while r < 8 {
    //             north.set(r * 8 + file);
    //             r += 1;
    //         }
    //         self.north_rays[index] = north;
    //
    //         // South ray (down)
    //         let mut south = BitBoard(0);
    //         let mut r = rank as i8 - 1;
    //         while r >= 0 {
    //             south.set(r as usize * 8 + file);
    //             r -= 1;
    //         }
    //         self.south_rays[index] = south;
    //
    //         // East ray (right)
    //         let mut east = BitBoard(0);
    //         let mut f = file + 1;
    //         while f < 8 {
    //             east.set(rank * 8 + f);
    //             f += 1;
    //         }
    //         self.east_rays[index] = east;
    //
    //         // West ray (left)
    //         let mut west = BitBoard(0);
    //         let mut f = file as i8 - 1;
    //         while f >= 0 {
    //             west.set(rank * 8 + f as usize);
    //             f -= 1;
    //         }
    //         self.west_rays[index] = west;
    //
    //         // Northeast ray (up-right)
    //         let mut northeast = BitBoard(0);
    //         let mut r = rank + 1;
    //         let mut f = file + 1;
    //         while r < 8 && f < 8 {
    //             northeast.set(r * 8 + f);
    //             r += 1;
    //             f += 1;
    //         }
    //         self.northeast_rays[index] = northeast;
    //
    //         // Southeast ray (down-right)
    //         let mut southeast = BitBoard(0);
    //         let mut r = rank as i8 - 1;
    //         let mut f = file + 1;
    //         while r >= 0 && f < 8 {
    //             southeast.set(r as usize * 8 + f);
    //             r -= 1;
    //             f += 1;
    //         }
    //         self.southeast_rays[index] = southeast;
    //
    //         // Southwest ray (down-left)
    //         let mut southwest = BitBoard(0);
    //         let mut r = rank as i8 - 1;
    //         let mut f = file as i8 - 1;
    //         while r >= 0 && f >= 0 {
    //             southwest.set(r as usize * 8 + f as usize);
    //             r -= 1;
    //             f -= 1;
    //         }
    //         self.southwest_rays[index] = southwest;
    //
    //         // Northwest ray (up-left)
    //         let mut northwest = BitBoard(0);
    //         let mut r = rank + 1;
    //         let mut f = file as i8 - 1;
    //         while r < 8 && f >= 0 {
    //             northwest.set(r * 8 + f as usize);
    //             r += 1;
    //             f -= 1;
    //         }
    //         self.northwest_rays[index] = northwest;
    //
    //         index += 1;
    //     }
    // }

    pub const fn get_rook_moves(
        &self,
        from: usize,
        ally_pieces: BitBoard,
        enemy_pieces: BitBoard,
    ) -> BitBoard {
        let mut moves = BitBoard(0);

        // Use the ray tables and add until blocker logic
        moves = moves
            .or(self.ray_until_blocker(self.north_rays[from], ally_pieces, enemy_pieces, true))
            .or(self.ray_until_blocker(self.south_rays[from], ally_pieces, enemy_pieces, false))
            .or(self.ray_until_blocker(self.east_rays[from], ally_pieces, enemy_pieces, true))
            .or(self.ray_until_blocker(self.west_rays[from], ally_pieces, enemy_pieces, false));

        moves
    }

    pub const fn get_bishop_moves(
        &self,
        from: usize,
        ally_pieces: BitBoard,
        enemy_pieces: BitBoard,
    ) -> BitBoard {
        let mut moves = BitBoard(0);

        // Use the ray tables and add until blocker logic
        moves = moves
            .or(self.ray_until_blocker(self.northeast_rays[from], ally_pieces, enemy_pieces, true))
            .or(self.ray_until_blocker(self.southeast_rays[from], ally_pieces, enemy_pieces, false))
            .or(self.ray_until_blocker(self.southwest_rays[from], ally_pieces, enemy_pieces, false))
            .or(self.ray_until_blocker(self.northwest_rays[from], ally_pieces, enemy_pieces, true));

        moves
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
                    moves = moves | single_push;

                    let double_push = self.white_pawn_double_pushes[from];
                    if double_push.0 != 0 && (double_push & occupied).0 == 0 {
                        moves = moves | double_push;
                    }
                }
            }
            Side::Black => {
                let single_push = self.black_pawn_pushes[from];
                if (single_push & occupied).0 == 0 {
                    moves = moves | single_push;

                    let double_push = self.black_pawn_double_pushes[from];
                    if double_push.0 != 0 && (double_push & occupied).0 == 0 {
                        moves = moves | double_push;
                    }
                }
            }
        }

        moves
    }

    pub fn get_ray(&self, from: usize, dir: i8) -> BitBoard {
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
}
