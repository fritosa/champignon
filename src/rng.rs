//! Injectable randomness: the game always draws through `g.rng` so that tests can fix the rolls.

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

/// Source of randomness used by the game logic (object safe: `Game` holds a `Box<dyn Rng>`).
/// `choice` and `sample` come from [`RandomExt`], available on every `Rng`.
pub trait Rng: Send {
    /// Float in [0, 1).
    fn random(&mut self) -> f64;

    /// Index in 0..n (n > 0), uniformly.
    fn below(&mut self, n: usize) -> usize;

    /// Float between a and b.
    fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random()
    }

    /// Integer in a..=b.
    fn randint(&mut self, a: i64, b: i64) -> i64 {
        a + self.below((b - a + 1) as usize) as i64
    }
}

/// Generic helpers on top of [`Rng`] (kept apart so that `Rng` stays object safe).
pub trait RandomExt: Rng {
    /// Random element of a non-empty slice.
    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    /// k distinct elements in random order (k <= len).
    fn sample<T: Clone>(&mut self, items: &[T], k: usize) -> Vec<T> {
        let mut pool: Vec<T> = items.to_vec();
        let mut picked = Vec::with_capacity(k);
        for _ in 0..k.min(items.len()) {
            let i = self.below(pool.len());
            picked.push(pool.swap_remove(i));
        }
        picked
    }
}

impl<R: Rng + ?Sized> RandomExt for R {}

/// Real generator, seedable (sim, art) or seeded from the system.
pub struct StdRandom(StdRng);

impl StdRandom {
    pub fn new() -> Self {
        StdRandom(rand::make_rng())
    }

    pub fn seeded(seed: u64) -> Self {
        StdRandom(StdRng::seed_from_u64(seed))
    }
}

impl Default for StdRandom {
    fn default() -> Self {
        Self::new()
    }
}

impl Rng for StdRandom {
    fn random(&mut self) -> f64 {
        self.0.random::<f64>()
    }

    fn below(&mut self, n: usize) -> usize {
        self.0.random_range(0..n)
    }
}

/// random() (hence uniform()) always returns the same value (0 → every roll "succeeds");
/// choice, sample and randint still draw from a generator seeded with 0.
pub struct FixedRandom {
    pub value: f64,
    inner: StdRandom,
}

impl FixedRandom {
    pub fn new(value: f64) -> Self {
        FixedRandom {
            value,
            inner: StdRandom::seeded(0),
        }
    }
}

impl Rng for FixedRandom {
    fn random(&mut self) -> f64 {
        self.value
    }

    fn below(&mut self, n: usize) -> usize {
        self.inner.below(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_is_deterministic() {
        let (mut a, mut b) = (StdRandom::seeded(3), StdRandom::seeded(3));
        let xs: Vec<f64> = (0..5).map(|_| a.random()).collect();
        let ys: Vec<f64> = (0..5).map(|_| b.random()).collect();
        assert_eq!(xs, ys);
        assert!(xs.iter().all(|x| (0.0..1.0).contains(x)));
    }

    #[test]
    fn helpers_stay_in_bounds() {
        let mut r = StdRandom::seeded(1);
        for _ in 0..200 {
            let u = r.uniform(40.0, 90.0);
            assert!((40.0..=90.0).contains(&u));
            let i = r.randint(-2, 2);
            assert!((-2..=2).contains(&i));
            assert!([1, 2, 3].contains(r.choice(&[1, 2, 3])));
        }
        let mut s = r.sample(&[1, 2, 3, 4, 5], 3);
        assert_eq!(s.len(), 3);
        s.sort();
        s.dedup();
        assert_eq!(s.len(), 3);
        assert_eq!(r.sample(&[1, 2], 5).len(), 2);
    }

    #[test]
    fn fixed_random() {
        let mut r = FixedRandom::new(0.5);
        assert_eq!(r.random(), 0.5);
        assert_eq!(r.uniform(0.0, 10.0), 5.0);
        let dynamic: &mut dyn Rng = &mut r;
        assert!([1, 2].contains(dynamic.choice(&[1, 2])));
    }
}
