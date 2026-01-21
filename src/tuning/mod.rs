pub mod gd_tuner;
pub mod macros;
pub mod params;
pub mod spsa_tuner;
#[cfg(test)]
mod tests;
pub mod texel;
pub mod trace;

pub trait Tunable {
    fn push_to_vector(&self, vec: &mut Vec<f64>);
    fn read_from_vector(vec: &[f64], idx: &mut usize) -> Self;
}
