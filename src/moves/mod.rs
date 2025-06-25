pub mod legacy;
pub mod move_gen;
pub mod move_info;
pub mod precomputed;

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
}
