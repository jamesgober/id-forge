//! Random bytes for the `uuid`, `ulid`, and `nanoid` modules.
//!
//! Every random bit comes from the operating system's random source
//! through `mod_rand::tier3` (`getrandom(2)` on Linux, `getentropy(3)`
//! on macOS, `BCryptGenRandom` on Windows, `/dev/urandom` on other
//! Unix systems). To keep the per-ID cost low, each thread reads
//! [`POOL_LEN`] bytes at a time into a private buffer and hands them
//! out in order; every byte is used once and zeroed as it is taken.
//!
//! If the OS random source fails, the draw panics instead of falling
//! back to a weaker generator. On supported platforms this does not
//! happen in practice: the source blocks rather than fail.

use std::cell::RefCell;

/// Bytes read from the OS per refill: 16 v4 UUIDs or ULIDs per call.
const POOL_LEN: usize = 256;

struct Pool {
    buf: [u8; POOL_LEN],
    pos: usize,
}

impl Pool {
    const fn empty() -> Self {
        Self {
            buf: [0; POOL_LEN],
            pos: POOL_LEN,
        }
    }

    #[inline]
    fn take<const N: usize>(&mut self) -> [u8; N] {
        if POOL_LEN - self.pos < N {
            refill(&mut self.buf);
            self.pos = 0;
        }
        let end = self.pos + N;
        let mut out = [0u8; N];
        out.copy_from_slice(&self.buf[self.pos..end]);
        self.buf[self.pos..end].fill(0);
        self.pos = end;
        out
    }
}

#[cold]
fn refill(buf: &mut [u8]) {
    if let Err(e) = mod_rand::tier3::fill_bytes(buf) {
        panic!("id-forge: the OS random source failed: {e}");
    }
}

thread_local! {
    static POOL: RefCell<Pool> = const { RefCell::new(Pool::empty()) };
}

/// Draw one `u64` from the OS random source.
#[cfg(any(feature = "ulid", feature = "nanoid"))]
pub(crate) fn next_u64() -> u64 {
    u64::from_le_bytes(POOL.with(|p| p.borrow_mut().take::<8>()))
}

/// Draw 16 bytes from the OS random source.
#[cfg(feature = "uuid")]
pub(crate) fn next_bytes_16() -> [u8; 16] {
    POOL.with(|p| p.borrow_mut().take::<16>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_differ() {
        let a = POOL.with(|p| p.borrow_mut().take::<16>());
        let b = POOL.with(|p| p.borrow_mut().take::<16>());
        assert_ne!(a, b);
        assert_ne!(a, [0; 16]);
    }

    #[test]
    fn draws_cross_refill_boundaries() {
        // 1000 draws of 16 and 8 bytes cross many refills; none may be
        // all zero (a missed refill would hand out wiped bytes).
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1000 {
            let a = POOL.with(|p| p.borrow_mut().take::<16>());
            assert!(seen.insert(a));
            let b = POOL.with(|p| p.borrow_mut().take::<8>());
            assert_ne!(b, [0; 8]);
        }
    }

    #[test]
    fn consumed_bytes_are_wiped() {
        POOL.with(|p| {
            let mut p = p.borrow_mut();
            let _ = p.take::<16>();
            let start = p.pos - 16;
            assert!(p.buf[start..p.pos].iter().all(|&b| b == 0));
        });
    }
}
