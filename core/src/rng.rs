//! Azar propio (xorshift64*) y ruido por hash: sin dependencias y reproducible con semilla.

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng((seed ^ 0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Flotante uniforme en [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}

/// Hash entero de tres valores: ruido estable sin guardar estado.
pub fn hash3(a: u32, b: u32, c: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77) ^ c.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    h
}

/// Ruido en [0, 1).
pub fn hf(a: u32, b: u32, c: u32) -> f32 {
    (hash3(a, b, c) >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let (mut a, mut b) = (Rng::new(1234), Rng::new(1234));
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let (mut a, mut b) = (Rng::new(1), Rng::new(2));
        let same = (0..100).filter(|_| a.next_u64() == b.next_u64()).count();
        assert_eq!(same, 0);
    }

    #[test]
    fn degenerate_seeds_do_not_get_stuck() {
        // xorshift con estado 0 se queda en 0 para siempre; el `| 1` lo evita.
        for seed in [0u64, 0x9E37_79B9_7F4A_7C15, 0x9E37_79B9_7F4A_7C14, u64::MAX] {
            let mut r = Rng::new(seed);
            let v: Vec<u64> = (0..8).map(|_| r.next_u64()).collect();
            assert!(v.iter().all(|&x| x != 0), "seed {seed:#x}: {v:?}");
            assert!(v.windows(2).all(|w| w[0] != w[1]), "seed {seed:#x}");
        }
    }

    #[test]
    fn f32_is_in_unit_interval_and_spread() {
        let mut r = Rng::new(7);
        let mut buckets = [0u32; 10];
        for _ in 0..10_000 {
            let v = r.f32();
            assert!((0.0..1.0).contains(&v), "{v}");
            buckets[(v * 10.0) as usize] += 1;
        }
        // Uniforme a grandes rasgos: cada décimo entre 800 y 1200 de 10000.
        assert!(buckets.iter().all(|&n| (800..1200).contains(&n)), "{buckets:?}");
    }

    #[test]
    fn range_chance_below() {
        let mut r = Rng::new(99);
        for _ in 0..5000 {
            let v = r.range(-2.0, 3.0);
            assert!((-2.0..3.0).contains(&v));
            assert!(r.below(7) < 7);
            assert_eq!(r.below(1), 0);
            assert_eq!(r.below(0), 0, "below(0) no debe dividir por cero");
            assert!(!r.chance(0.0));
            assert!(r.chance(1.0));
        }
        // Rango degenerado.
        assert_eq!(r.range(5.0, 5.0), 5.0);
        // below cubre todos los valores.
        let mut seen = [false; 5];
        for _ in 0..500 {
            seen[r.below(5)] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn hash_is_stable_and_noise_in_range() {
        assert_eq!(hash3(1, 2, 3), hash3(1, 2, 3));
        assert_ne!(hash3(1, 2, 3), hash3(3, 2, 1));
        assert_ne!(hash3(0, 0, 0), hash3(0, 0, 1));
        for a in 0..50u32 {
            for b in [0u32, 1, 1000, u32::MAX] {
                let v = hf(a, b, a ^ b);
                assert!((0.0..1.0).contains(&v), "{v}");
            }
        }
    }
}
