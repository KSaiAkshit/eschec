#[derive(Copy, Clone, Debug, Default)]
pub struct Prng {
    state: u64,
}

impl Prng {
    pub fn init(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn rand(&mut self) -> u64 {
        self.next_u64()
    }

    pub fn sparse_rand(&mut self) -> u64 {
        self.next_u64() & self.next_u64() & self.next_u64()
    }

    pub fn singular_bit(&mut self) -> u64 {
        let random_shift = self.rand() % 64;
        1u64 << random_shift
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const ROUNDS: u32 = 100;
    const MUTS: u32 = 1000;

    #[test]
    fn check_bit_displacement() {
        let mut seeder = Prng::init(10300014);
        let mut acc = [0u32; 64];
        for _ in 0..ROUNDS {
            let mut prng = Prng::init(seeder.rand());
            for _ in 0..MUTS {
                add_to_bit_counts(prng.singular_bit(), &mut acc);
            }
        }

        let max = *acc.iter().max().unwrap();
        acc.iter_mut().enumerate().for_each(|(i, m)| {
            *m *= 100;
            *m /= max;
            println!("{i} : {m}");
        });

        let sum: u32 = acc.iter().sum();
        println!("avg: {}", sum / 64);
        if acc.contains(&0u32) {
            panic!("There was a bit that was choosen 0 times.")
        }
    }

    fn add_to_bit_counts(mut num: u64, acc: &mut [u32; 64]) {
        while num != 0 {
            let i = num.trailing_zeros();
            acc[i as usize] += 1;
            num &= !((1u64) << i);
        }
    }
}
