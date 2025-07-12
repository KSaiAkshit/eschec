pub mod attack_data;
pub mod move_gen;
pub mod move_info;
pub mod precomputed;
pub mod pseudo_legal;

#[cfg(test)]
mod tests;

/// First 4 are orhtogonal, rest are diagonal
///  (N, S, W, E, NW, SE, NE, SW)
pub struct Direction;
impl Direction {
    pub const NORTH: i8 = 8;
    pub const SOUTH: i8 = -8;
    pub const WEST: i8 = -1;
    pub const EAST: i8 = 1;
    pub const NORTHWEST: i8 = 7;
    pub const SOUTHEAST: i8 = -7;
    pub const NORTHEAST: i8 = 9;
    pub const SOUTHWEST: i8 = -9;

    pub const ORTHO: [i8; 4] = [Self::NORTH, Self::SOUTH, Self::WEST, Self::EAST];
    pub const DIAG: [i8; 4] = [
        Self::NORTHEAST,
        Self::SOUTHEAST,
        Self::SOUTHWEST,
        Self::NORTHWEST,
    ];
    pub const ALL: [i8; 8] = [
        Self::NORTH,
        Self::SOUTH,
        Self::WEST,
        Self::EAST,
        Self::NORTHEAST,
        Self::SOUTHEAST,
        Self::SOUTHWEST,
        Self::NORTHWEST,
    ];

    #[inline(always)]
    pub const fn is_ortho(dir: i8) -> bool {
        matches!(
            dir,
            Direction::NORTH | Direction::SOUTH | Direction::EAST | Direction::WEST
        )
    }

    #[inline(always)]
    pub const fn is_diag(dir: i8) -> bool {
        matches!(
            dir,
            Direction::NORTHEAST
                | Direction::SOUTHEAST
                | Direction::SOUTHWEST
                | Direction::NORTHWEST
        )
    }

    pub const fn get_dir(from: usize, to: usize) -> i8 {
        let rank_diff = (to / 8) as i8 - (from / 8) as i8;
        let file_diff = (to % 8) as i8 - (from % 8) as i8;

        if rank_diff == 0 {
            // Horizontal
            if file_diff > 0 {
                Self::EAST
            } else {
                Self::WEST
            }
        } else if file_diff == 0 {
            // Vertical
            if rank_diff > 0 {
                Self::NORTH
            } else {
                Self::SOUTH
            }
        } else if rank_diff == file_diff {
            // Diagonal
            if rank_diff > 0 {
                Self::NORTHEAST
            } else {
                Self::SOUTHWEST
            }
        } else if rank_diff == -file_diff {
            // Anti-diagonal
            if rank_diff > 0 {
                Self::NORTHWEST
            } else {
                Self::SOUTHEAST
            }
        } else {
            0 // Not a straight line
        }
    }
}
