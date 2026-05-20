//! Inline xoshiro256\*\* generator shared by the `uuid`, `ulid`, and
//! `nanoid` modules.
//!
//! The seed is derived from process ID, wall-clock nanoseconds, and a
//! per-process counter run through SplitMix64. The generator state
//! lives in a thread-local; `next_u64` after the first call is a
//! handful of register-only operations with no syscall and no
//! contention.
//!
//! **Not cryptographically secure.** Callers needing CSPRNG output
//! should compose `id-forge` with their own source.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static RNG: RefCell<Xoshiro256SS> = RefCell::new(Xoshiro256SS::from_entropy());
}

/// Draw one `u64` from the thread-local RNG.
pub(crate) fn next_u64() -> u64 {
    RNG.with(|cell| cell.borrow_mut().next_u64())
}

/// Draw 16 random bytes — two `u64` words written big-endian.
pub(crate) fn next_bytes_16() -> [u8; 16] {
    RNG.with(|cell| {
        let mut r = cell.borrow_mut();
        let a = r.next_u64();
        let b = r.next_u64();
        let mut out = [0u8; 16];
        out[0..8].copy_from_slice(&a.to_be_bytes());
        out[8..16].copy_from_slice(&b.to_be_bytes());
        out
    })
}

pub(crate) struct Xoshiro256SS {
    s: [u64; 4],
}

impl Xoshiro256SS {
    pub(crate) fn from_entropy() -> Self {
        static SEED_COUNTER: AtomicU64 = AtomicU64::new(0);
        let pid = std::process::id() as u64;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let counter = SEED_COUNTER.fetch_add(1, Ordering::Relaxed);
        let seed = pid
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(nanos)
            .wrapping_add(counter.wrapping_mul(0xBF58_476D_1CE4_E5B9));
        Self::from_seed(seed)
    }

    pub(crate) fn from_seed(mut seed: u64) -> Self {
        let mut s = [0u64; 4];
        for slot in &mut s {
            seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = seed;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *slot = z ^ (z >> 31);
        }
        if s == [0; 4] {
            s[0] = 1;
        }
        Self { s }
    }

    #[inline]
    pub(crate) fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_state_is_nonzero() {
        let mut r = Xoshiro256SS::from_seed(0);
        let a = r.next_u64();
        let b = r.next_u64();
        assert_ne!(a, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Xoshiro256SS::from_seed(1);
        let mut b = Xoshiro256SS::from_seed(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn entropy_draws_differ() {
        assert_ne!(next_u64(), next_u64());
    }

    #[test]
    fn bytes_16_differs_across_calls() {
        assert_ne!(next_bytes_16(), next_bytes_16());
    }
}
