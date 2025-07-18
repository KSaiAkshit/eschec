pub mod attack_data;
pub mod move_gen;
pub mod move_info;
pub mod precomputed;

use std::ops::{Add, Mul, Neg, Sub};

#[cfg(test)]
mod tests;

/// First 4 are orhtogonal, rest are diagonal
///  (N, S, W, E, NW, SE, NE, SW)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct Direction(i8);
impl Direction {
    pub const NORTH: Direction = Direction(8);
    pub const SOUTH: Direction = Direction(-8);
    pub const WEST: Direction = Direction(-1);
    pub const EAST: Direction = Direction(1);
    pub const NORTHWEST: Direction = Direction(7);
    pub const SOUTHEAST: Direction = Direction(-7);
    pub const NORTHEAST: Direction = Direction(9);
    pub const SOUTHWEST: Direction = Direction(-9);

    pub const ORTHO: [Direction; 4] = [Self::NORTH, Self::SOUTH, Self::WEST, Self::EAST];
    pub const DIAG: [Direction; 4] = [
        Self::NORTHEAST,
        Self::SOUTHEAST,
        Self::SOUTHWEST,
        Self::NORTHWEST,
    ];
    pub const ALL: [Direction; 8] = [
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
    pub const fn value(&self) -> i8 {
        self.0
    }

    #[inline(always)]
    pub const fn is_ortho(dir: Direction) -> bool {
        matches!(dir.0, 8 | -8 | -1 | 1)
    }

    #[inline(always)]
    pub const fn is_diag(dir: Direction) -> bool {
        matches!(dir.0, 7 | -7 | 9 | -9)
    }

    pub const fn get_dir(from: usize, to: usize) -> Direction {
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
            Direction(0) // Not a straight line
        }
    }
}

impl From<i8> for Direction {
    fn from(item: i8) -> Self {
        Direction(item)
    }
}

impl Neg for Direction {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Direction(-self.0)
    }
}

impl Add<Direction> for i8 {
    type Output = i8;
    fn add(self, rhs: Direction) -> Self::Output {
        self + rhs.0
    }
}

impl Add<i8> for Direction {
    type Output = i8;
    fn add(self, rhs: i8) -> Self::Output {
        self.0 + rhs
    }
}

impl Sub<Direction> for i8 {
    type Output = i8;
    fn sub(self, rhs: Direction) -> Self::Output {
        self - rhs.0
    }
}

impl Mul<i8> for Direction {
    type Output = Direction;
    fn mul(self, rhs: i8) -> Self::Output {
        Direction(self.0 * rhs)
    }
}
