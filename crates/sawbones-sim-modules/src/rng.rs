//! A small seeded generator, so every run sees the same sequence.

/// An xorshift generator. Not for security; for repeatable scenarios.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A generator for `seed`. Any seed, including 0, gives a usable stream.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Rng(mix(seed) | 1)
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A number below `n`, or 0 if `n` is 0.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }

    /// A random byte.
    pub fn byte(&mut self) -> u8 {
        self.next_u64().to_le_bytes()[0]
    }

    /// `N` random bytes.
    pub fn array<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        for b in &mut out {
            *b = self.byte();
        }
        out
    }
}

/// A stateless 64-bit mixer (splitmix64's finaliser).
#[must_use]
pub fn mix(x: u64) -> u64 {
    let mut x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}
