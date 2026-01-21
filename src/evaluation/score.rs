use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::tuning::Tunable;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Type safe wrapper for game phase
/// The value is scaled from 0 (midgame) to 256 (full endgame)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Phase(pub i32);

/// Score that holds seperate values for midgame and endgame
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize,
)]
pub struct Score {
    // MidGame score
    pub mg: i32,
    // EndGame score
    pub eg: i32,
}

impl Display for Score {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MG: {}, EG: {}", self.mg, self.eg)
    }
}

pub const fn const_zip(mg: [i32; NUM_SQUARES], eg: [i32; NUM_SQUARES]) -> [Score; NUM_SQUARES] {
    let mut result = [Score::new(0, 0); NUM_SQUARES];
    let mut i = 0;
    while i < NUM_SQUARES {
        result[i] = Score::new(mg[i], eg[i]);
        i += 1;
    }
    result
}

impl Score {
    #[inline(always)]
    /// Creates a new score with distinct midgame and endgame values
    pub const fn new(mg: i32, eg: i32) -> Self {
        Self { mg, eg }
    }

    /// Creates a score where midgame and endgame values are the same
    /// Useful for evaluation terms that are not phase-dependent
    #[inline(always)]
    pub const fn splat(score: i32) -> Self {
        Self {
            mg: score,
            eg: score,
        }
    }

    #[inline]
    pub const fn taper(&self, phase: Phase) -> i32 {
        let mg_w = ENDGAME_PHASE - phase.0;
        let eg_w = phase.0;
        ((self.mg * mg_w) + (self.eg * eg_w)) / ENDGAME_PHASE
    }
}

impl Tunable for Score {
    fn push_to_vector(&self, vec: &mut Vec<f64>) {
        vec.push(self.mg as f64);
        vec.push(self.eg as f64);
    }

    fn read_from_vector(vec: &[f64], idx: &mut usize) -> Self {
        let mg = vec[*idx] as i32;
        let eg = vec[*idx + 1] as i32;
        *idx += 2;
        Score::new(mg, eg)
    }
}

impl<const N: usize> Tunable for [Score; N] {
    fn push_to_vector(&self, vec: &mut Vec<f64>) {
        for score in self {
            score.push_to_vector(vec);
        }
    }

    fn read_from_vector(vec: &[f64], idx: &mut usize) -> Self {
        std::array::from_fn(|_| Score::read_from_vector(vec, idx))
    }
}

impl Add for Score {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            mg: self.mg + rhs.mg,
            eg: self.eg + rhs.eg,
        }
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.mg += rhs.mg;
        self.eg += rhs.eg;
    }
}

impl Sub for Score {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            mg: self.mg - rhs.mg,
            eg: self.eg - rhs.eg,
        }
    }
}

impl SubAssign for Score {
    fn sub_assign(&mut self, rhs: Self) {
        self.mg -= rhs.mg;
        self.eg -= rhs.eg;
    }
}

impl Neg for Score {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            mg: -self.mg,
            eg: -self.eg,
        }
    }
}

impl Mul<i32> for Score {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Self {
            mg: self.mg * rhs,
            eg: self.eg * rhs,
        }
    }
}

impl Div<i32> for Score {
    type Output = Self;

    fn div(self, rhs: i32) -> Self::Output {
        Self {
            mg: self.mg / rhs,
            eg: self.eg / rhs,
        }
    }
}
