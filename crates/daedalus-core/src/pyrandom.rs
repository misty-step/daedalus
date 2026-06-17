//! Faithful reimplementation of CPython's `random.Random` (MT19937), enough to
//! reproduce `random.Random(seed).shuffle(...)` exactly.
//!
//! The search loop (`runner/loop.py`) and seeder (`runner/seed.py`) shuffle with
//! a seeded `random.Random`, and `tests/test_loop.py` relies on the default
//! `random.Random(0)` trajectory — so a byte-faithful port must reproduce
//! CPython's PRNG bit-for-bit. Only the integer paths (`getrandbits`,
//! `_randbelow`, `shuffle`) are implemented; the float `random()` is not needed.
//!
//! Verified against CPython in `tests/parity_pyrandom.rs`.

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

/// A Mersenne-Twister PRNG seeded and stepped exactly like CPython's
/// `random.Random`.
pub struct PyRandom {
    mt: [u32; N],
    index: usize,
}

impl PyRandom {
    /// Seed like CPython `random.Random(seed)` for a non-negative integer seed
    /// (the seed is split into little-endian 32-bit words, then `init_by_array`).
    pub fn new(seed: u64) -> Self {
        let mut r = PyRandom {
            mt: [0; N],
            index: N + 1,
        };
        let key: Vec<u32> = if seed == 0 {
            vec![0]
        } else {
            let mut k = Vec::new();
            let mut s = seed;
            while s > 0 {
                k.push((s & 0xffff_ffff) as u32);
                s >>= 32;
            }
            k
        };
        r.init_by_array(&key);
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            self.mt[i] = 1_812_433_253u32
                .wrapping_mul(self.mt[i - 1] ^ (self.mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        self.index = N;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let mut i = 1usize;
        let mut j = 0usize;
        let mut k = N.max(key.len());
        while k > 0 {
            self.mt[i] = (self.mt[i]
                ^ (self.mt[i - 1] ^ (self.mt[i - 1] >> 30)).wrapping_mul(1_664_525))
            .wrapping_add(key[j])
            .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            self.mt[i] = (self.mt[i]
                ^ (self.mt[i - 1] ^ (self.mt[i - 1] >> 30)).wrapping_mul(1_566_083_941))
            .wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        self.mt[0] = 0x8000_0000;
    }

    fn genrand_uint32(&mut self) -> u32 {
        if self.index >= N {
            for kk in 0..(N - M) {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] = self.mt[kk + M] ^ (y >> 1) ^ if y & 1 != 0 { MATRIX_A } else { 0 };
            }
            for kk in (N - M)..(N - 1) {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] =
                    self.mt[kk + M - N] ^ (y >> 1) ^ if y & 1 != 0 { MATRIX_A } else { 0 };
            }
            let y = (self.mt[N - 1] & UPPER_MASK) | (self.mt[0] & LOWER_MASK);
            self.mt[N - 1] = self.mt[M - 1] ^ (y >> 1) ^ if y & 1 != 0 { MATRIX_A } else { 0 };
            self.index = 0;
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// CPython `getrandbits(k)`.
    pub fn getrandbits(&mut self, k: u32) -> u64 {
        if k == 0 {
            return 0;
        }
        if k <= 32 {
            return (self.genrand_uint32() >> (32 - k)) as u64;
        }
        let mut result: u64 = 0;
        let mut bits = k;
        let mut shift = 0;
        while bits > 0 {
            let take = bits.min(32);
            let word = (self.genrand_uint32() >> (32 - take)) as u64;
            result |= word << shift;
            shift += 32;
            bits -= take;
        }
        result
    }

    /// CPython `Random._randbelow(n)` for `n > 0`.
    pub fn randbelow(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let k = bit_length(n);
        let mut r = self.getrandbits(k);
        while r >= n {
            r = self.getrandbits(k);
        }
        r
    }

    /// CPython `random.shuffle(x)`: Fisher–Yates driven by `_randbelow`.
    pub fn shuffle<T>(&mut self, x: &mut [T]) {
        let n = x.len();
        for i in (1..n).rev() {
            let j = self.randbelow((i + 1) as u64) as usize;
            x.swap(i, j);
        }
    }
}

/// Python `int.bit_length()`.
fn bit_length(n: u64) -> u32 {
    if n == 0 {
        0
    } else {
        64 - n.leading_zeros()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_length_matches_python() {
        assert_eq!(bit_length(0), 0);
        assert_eq!(bit_length(1), 1);
        assert_eq!(bit_length(2), 2);
        assert_eq!(bit_length(3), 2);
        assert_eq!(bit_length(4), 3);
        assert_eq!(bit_length(255), 8);
        assert_eq!(bit_length(256), 9);
    }

    #[test]
    fn shuffle_is_deterministic_per_seed() {
        // Two PyRandoms with the same seed shuffle identically.
        let mut a: Vec<u32> = (0..10).collect();
        let mut b = a.clone();
        PyRandom::new(0).shuffle(&mut a);
        PyRandom::new(0).shuffle(&mut b);
        assert_eq!(a, b);
        // A different seed (usually) gives a different order; at least it runs.
        let mut c: Vec<u32> = (0..10).collect();
        PyRandom::new(1).shuffle(&mut c);
        assert_eq!(c.len(), 10);
    }
}
