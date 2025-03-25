use crate::board::components::Square;

use super::components::{BitBoard, Piece};

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct Moves {
    piece: Piece,
    pub attack_bb: Vec<BitBoard>,
}

pub struct Direction;
impl Direction {
    pub const NORTH: i8 = 8;
    pub const SOUTH: i8 = -8;
    pub const WEST: i8 = -1;
    pub const EAST: i8 = 1;
    pub const NORTHWEST: i8 = -7;
    pub const SOUTHEAST: i8 = 7;
    pub const NORTHEAST: i8 = 9;
    pub const SOUTHWEST: i8 = -9;

    pub const ORTHO: [i8; 4] = [Self::NORTH, Self::SOUTH, Self::WEST, Self::EAST];
}

impl Moves {
    /// First 4 are orhtogonal, rest are diagonal
    ///                                (N, S, W, E, NW, SE, NE, SW)
    pub fn new(piece: Piece) -> Self {
        let moves = Self::default();
        match piece {
            Piece::Knight => moves.gen_knight_moves(),
            Piece::Rook => moves.gen_rook_moves(),
            _ => Moves::default(),
        }
    }

    pub fn gen_rook_moves(self) -> Self {
        let mut attack_bb = vec![BitBoard(0); 64];
        (0..64).for_each(|index| {
            let square = Square(index);
            let mut rook_moves = BitBoard(0);
            let (file, rank) = square.coords();

            // Vertical
            for f in 0..8 {
                if f != file {
                    let target_index = f * 8 + rank;
                    // dbg!(index, f, rank, target_index);
                    let target_bb = BitBoard(1 << target_index);
                    rook_moves = rook_moves | target_bb;
                }
            }

            // // Horizontal
            for r in 0..8 {
                if r != rank {
                    let target_index = r + file * 8;
                    let target_bb = BitBoard(1 << target_index);
                    rook_moves = rook_moves | target_bb;
                }
            }
            attack_bb[index] = rook_moves;
        });
        Self {
            piece: Piece::Rook,
            attack_bb,
        }
    }

    pub fn gen_knight_moves(self) -> Self {
        let mut attack_bb = vec![BitBoard(0); 64];
        let knight_offsets: [i8; 8] = [-17, -15, -10, -6, 6, 10, 15, 17];

        (0..64).for_each(|index| {
            let mut knight_moves = BitBoard(0);

            // Iterate over each knight offset
            for &offset in knight_offsets.iter() {
                let target_index = index as i8 + offset;
                if (0..64).contains(&target_index) {
                    // Calculate the coordinates of the knight square and target square
                    let knight_square_y = index / 8;
                    let knight_square_x = index % 8;
                    let target_square_y = target_index as usize / 8;
                    let target_square_x = target_index as usize % 8;

                    // Calculate the maximum coordinate move distance
                    let max_coord_move_dst = i8::max(
                        (knight_square_x as i8 - target_square_x as i8).abs(),
                        (knight_square_y as i8 - target_square_y as i8).abs(),
                    );

                    // If the maximum coordinate move distance is 2, the move is valid
                    if max_coord_move_dst == 2 {
                        let target_bb = BitBoard(1 << target_index);

                        // Add the target square bitboard to knight_moves
                        knight_moves = knight_moves | target_bb;
                    } // // Generate a bitboard with only the target square set
                }
            }
            attack_bb[index] = knight_moves;
        });

        Moves {
            piece: Piece::Knight, // Assuming you have a Piece enum
            attack_bb,
        }
    }
}
