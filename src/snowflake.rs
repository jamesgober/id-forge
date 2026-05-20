//! # Snowflake ID generation
//!
//! 64-bit IDs in the Twitter Snowflake layout: a 41-bit millisecond
//! timestamp offset from a configurable epoch, a 10-bit worker ID,
//! and a 12-bit per-millisecond sequence.
//!
//! ```text
//!  63                   22         12         0
//!  +---------------------+----------+----------+
//!  | 41 bits ms offset   | 10 bits  | 12 bits  |
//!  | (since epoch_ms)    | worker   | sequence |
//!  +---------------------+----------+----------+
//! ```
//!
//! IDs are strictly monotonic within a single worker. When 4096 IDs
//! are minted in the same millisecond the generator blocks (microsleep
//! loop) until the wall clock advances. When the wall clock moves
//! backward, [`Snowflake::try_next_id`] returns
//! `Err(`[`ClockSkew`]`)` and [`Snowflake::next_id`] panics — the spec
//! forbids issuing IDs whose timestamps could collide with previously
//! issued ones.
//!
//! ```
//! use id_forge::snowflake::Snowflake;
//!
//! let gen = Snowflake::new(1);
//! let a = gen.next_id();
//! let b = gen.next_id();
//! assert!(b > a);
//! let (ts_offset, worker, seq) = Snowflake::parts(b);
//! assert_eq!(worker, 1);
//! assert!(ts_offset > 0);
//! assert!(seq <= 4095);
//! ```

use core::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Default epoch (2026-01-01T00:00:00Z in milliseconds since UNIX epoch).
pub const DEFAULT_EPOCH_MS: u64 = 1_767_225_600_000;

/// Number of bits assigned to the sequence field in an ID.
pub const SEQUENCE_BITS: u32 = 12;
/// Number of bits assigned to the worker field in an ID.
pub const WORKER_BITS: u32 = 10;
/// Number of bits assigned to the timestamp offset in an ID.
pub const TIMESTAMP_BITS: u32 = 41;

const SEQUENCE_MASK: u64 = (1 << SEQUENCE_BITS) - 1;
const WORKER_MASK: u64 = (1 << WORKER_BITS) - 1;
const TIMESTAMP_MASK: u64 = (1 << TIMESTAMP_BITS) - 1;

const WORKER_SHIFT: u32 = SEQUENCE_BITS;
const TIMESTAMP_SHIFT: u32 = SEQUENCE_BITS + WORKER_BITS;

// Packed state layout: bits 0..13 hold the next sequence to assign
// (0..=4096; the 4096 sentinel means "this millisecond is exhausted"),
// bits 13..54 hold the last-seen millisecond offset (41 bits).
const STATE_SEQ_BITS: u32 = 13;
const STATE_SEQ_MASK: u64 = (1 << STATE_SEQ_BITS) - 1;
const STATE_SEQ_EXHAUSTED: u64 = SEQUENCE_MASK + 1; // 4096

/// Snowflake ID generator.
///
/// Holds the worker ID, the epoch, and the packed `(last_ms, next_seq)`
/// state used to mint monotonic IDs lock-free.
///
/// # Example
///
/// ```
/// use id_forge::snowflake::Snowflake;
///
/// let gen = Snowflake::new(7);
/// let id = gen.next_id();
/// assert_eq!(Snowflake::parts(id).1, 7);
/// ```
#[derive(Debug)]
pub struct Snowflake {
    worker_id: u16,
    epoch_ms: u64,
    state: AtomicU64,
}

impl Snowflake {
    /// Build a new generator with the given worker ID (0-1023) and
    /// the default 2026-01-01 epoch.
    ///
    /// Worker IDs above 1023 are silently clamped to 10 bits.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::snowflake::Snowflake;
    ///
    /// let gen = Snowflake::new(42);
    /// assert_eq!(gen.worker_id(), 42);
    /// ```
    pub const fn new(worker_id: u16) -> Self {
        Self::with_epoch(worker_id, DEFAULT_EPOCH_MS)
    }

    /// Build a new generator with a custom epoch (milliseconds since
    /// the UNIX epoch).
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::snowflake::Snowflake;
    ///
    /// // Twitter's original Snowflake epoch.
    /// let gen = Snowflake::with_epoch(1, 1_288_834_974_657);
    /// assert_eq!(gen.epoch_ms(), 1_288_834_974_657);
    /// ```
    pub const fn with_epoch(worker_id: u16, epoch_ms: u64) -> Self {
        Self {
            worker_id: (worker_id as u64 & WORKER_MASK) as u16,
            epoch_ms,
            state: AtomicU64::new(0),
        }
    }

    /// The worker ID this generator was built with, clamped to 10 bits.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::snowflake::Snowflake;
    ///
    /// assert_eq!(Snowflake::new(42).worker_id(), 42);
    /// assert_eq!(Snowflake::new(0xffff).worker_id(), 0x3ff);  // clamped
    /// ```
    pub const fn worker_id(&self) -> u16 {
        self.worker_id
    }

    /// The epoch this generator subtracts from the wall clock.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::snowflake::{Snowflake, DEFAULT_EPOCH_MS};
    ///
    /// assert_eq!(Snowflake::new(1).epoch_ms(), DEFAULT_EPOCH_MS);
    /// ```
    pub const fn epoch_ms(&self) -> u64 {
        self.epoch_ms
    }

    /// Generate the next ID, returning `Err` if the wall clock has
    /// moved backward since the previous call.
    ///
    /// When 4096 IDs have been issued in the same millisecond, this
    /// method blocks in a microsecond sleep loop until the wall clock
    /// advances. It does **not** spin on a busy loop.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::snowflake::Snowflake;
    ///
    /// let gen = Snowflake::new(1);
    /// let id = gen.try_next_id().expect("clock should not run backward");
    /// assert!(id > 0);
    /// ```
    pub fn try_next_id(&self) -> Result<u64, ClockSkew> {
        loop {
            let cur = self.state.load(Ordering::Acquire);
            let last_ms = cur >> STATE_SEQ_BITS;
            let next_seq = cur & STATE_SEQ_MASK;

            let now = current_offset_ms(self.epoch_ms);
            if now < last_ms {
                return Err(ClockSkew {
                    last_ms,
                    now_ms: now,
                });
            }

            let (use_ms, assigned, new_next_seq) = if now == last_ms {
                if next_seq >= STATE_SEQ_EXHAUSTED {
                    sleep_until_after(self.epoch_ms, last_ms);
                    continue;
                }
                (last_ms, next_seq, next_seq + 1)
            } else {
                (now, 0u64, 1u64)
            };

            let new_state = (use_ms << STATE_SEQ_BITS) | new_next_seq;
            if self
                .state
                .compare_exchange(cur, new_state, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                let id = (use_ms << TIMESTAMP_SHIFT)
                    | ((self.worker_id as u64) << WORKER_SHIFT)
                    | assigned;
                return Ok(id);
            }
        }
    }

    /// Generate the next ID. Panics if the wall clock has moved
    /// backward since the previous call.
    ///
    /// Use [`Snowflake::try_next_id`] when callers need to recover
    /// from clock skew (e.g. to surface it as a service-level error
    /// rather than crash the process).
    ///
    /// # Panics
    ///
    /// Panics with a [`ClockSkew`] description if the wall clock has
    /// regressed below the most recently issued ID's timestamp.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::snowflake::Snowflake;
    ///
    /// let gen = Snowflake::new(1);
    /// let id = gen.next_id();
    /// assert!(id > 0);
    /// ```
    pub fn next_id(&self) -> u64 {
        match self.try_next_id() {
            Ok(id) => id,
            Err(e) => panic!("snowflake: clock moved backward ({e})"),
        }
    }

    /// Decompose an ID minted by any Snowflake generator into its
    /// `(timestamp_offset_ms, worker_id, sequence)` parts.
    ///
    /// The first element is the millisecond offset from whatever epoch
    /// the originating generator was built with. To recover the
    /// wall-clock millisecond, add the generator's [`epoch_ms`](Self::epoch_ms).
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::snowflake::Snowflake;
    ///
    /// let gen = Snowflake::new(7);
    /// let id = gen.next_id();
    /// let (ts_offset, worker, seq) = Snowflake::parts(id);
    /// assert_eq!(worker, 7);
    /// assert!(seq <= 4095);
    /// let wall_ms = ts_offset + gen.epoch_ms();
    /// assert!(wall_ms > gen.epoch_ms());
    /// ```
    pub const fn parts(id: u64) -> (u64, u16, u16) {
        let timestamp_offset = (id >> TIMESTAMP_SHIFT) & TIMESTAMP_MASK;
        let worker = ((id >> WORKER_SHIFT) & WORKER_MASK) as u16;
        let sequence = (id & SEQUENCE_MASK) as u16;
        (timestamp_offset, worker, sequence)
    }
}

/// Error returned by [`Snowflake::try_next_id`] when the system clock
/// has moved backward since the most recent ID was issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSkew {
    /// The most recent millisecond offset (since the epoch) at which
    /// the generator successfully issued an ID.
    pub last_ms: u64,
    /// The current millisecond offset reported by the wall clock,
    /// which is strictly less than `last_ms`.
    pub now_ms: u64,
}

impl fmt::Display for ClockSkew {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "clock moved backward: last issued at offset {} ms, now at offset {} ms",
            self.last_ms, self.now_ms
        )
    }
}

impl std::error::Error for ClockSkew {}

fn current_offset_ms(epoch_ms: u64) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    now.saturating_sub(epoch_ms) & TIMESTAMP_MASK
}

fn sleep_until_after(epoch_ms: u64, last_ms: u64) {
    loop {
        if current_offset_ms(epoch_ms) > last_ms {
            return;
        }
        std::thread::sleep(Duration::from_micros(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn next_id_produces_value() {
        let gen = Snowflake::new(1);
        assert!(gen.next_id() > 0);
    }

    #[test]
    fn worker_id_clamped() {
        let gen = Snowflake::new(0xffff);
        assert_eq!(gen.worker_id(), 0x3ff);
        let id = gen.next_id();
        assert_eq!(Snowflake::parts(id).1, 0x3ff);
    }

    #[test]
    fn worker_field_extracts() {
        let gen = Snowflake::new(42);
        let id = gen.next_id();
        let (_, worker, _) = Snowflake::parts(id);
        assert_eq!(worker, 42);
    }

    #[test]
    fn monotonic_in_burst() {
        let gen = Snowflake::new(1);
        let mut prev = gen.next_id();
        for _ in 0..10_000 {
            let cur = gen.next_id();
            assert!(cur > prev, "expected {cur} > {prev}");
            prev = cur;
        }
    }

    #[test]
    fn all_unique_in_burst() {
        let gen = Snowflake::new(1);
        let mut set = HashSet::new();
        for _ in 0..50_000 {
            let id = gen.next_id();
            assert!(set.insert(id));
        }
    }

    #[test]
    fn parts_round_trip() {
        let gen = Snowflake::with_epoch(7, DEFAULT_EPOCH_MS);
        let id = gen.next_id();
        let (ts, worker, seq) = Snowflake::parts(id);
        assert_eq!(worker, 7);
        let reassembled = (ts << TIMESTAMP_SHIFT) | ((worker as u64) << WORKER_SHIFT) | seq as u64;
        assert_eq!(reassembled, id);
    }

    #[test]
    fn sequence_resets_each_ms() {
        let gen = Snowflake::new(1);
        let _ = gen.next_id();
        thread::sleep(Duration::from_millis(3));
        let id_after_sleep = gen.next_id();
        let (_, _, seq) = Snowflake::parts(id_after_sleep);
        assert_eq!(seq, 0, "first ID of a fresh ms must have sequence 0");
    }

    #[test]
    fn sequence_exhaustion_blocks_until_next_ms() {
        // Pre-load state to simulate an exhausted millisecond.
        let gen = Snowflake::new(1);
        let now = current_offset_ms(gen.epoch_ms);
        let exhausted_state = (now << STATE_SEQ_BITS) | STATE_SEQ_EXHAUSTED;
        gen.state.store(exhausted_state, Ordering::Release);

        let start = SystemTime::now();
        let id = gen.next_id();
        let elapsed = SystemTime::now().duration_since(start).unwrap();

        let (ts, _, seq) = Snowflake::parts(id);
        assert!(ts > now, "new ID must be in a later millisecond");
        assert_eq!(seq, 0);
        assert!(
            elapsed < Duration::from_millis(50),
            "block should be roughly one ms, got {elapsed:?}"
        );
    }

    #[test]
    fn clock_skew_reported_via_result() {
        let gen = Snowflake::new(1);
        // Force the state to claim we've already issued an ID
        // far in the future relative to the wall clock.
        let future_ms = current_offset_ms(gen.epoch_ms) + 5_000;
        gen.state
            .store(future_ms << STATE_SEQ_BITS, Ordering::Release);

        match gen.try_next_id() {
            Err(ClockSkew { last_ms, now_ms }) => {
                assert_eq!(last_ms, future_ms);
                assert!(now_ms < last_ms);
            }
            Ok(id) => panic!("expected ClockSkew, got id {id}"),
        }
    }

    #[test]
    #[should_panic(expected = "clock moved backward")]
    fn next_id_panics_on_clock_skew() {
        let gen = Snowflake::new(1);
        let future_ms = current_offset_ms(gen.epoch_ms) + 5_000;
        gen.state
            .store(future_ms << STATE_SEQ_BITS, Ordering::Release);
        let _ = gen.next_id();
    }

    #[test]
    fn multi_thread_all_unique() {
        let gen = Arc::new(Snowflake::new(3));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let g = Arc::clone(&gen);
            handles.push(thread::spawn(move || {
                let mut local = Vec::with_capacity(2000);
                for _ in 0..2000 {
                    local.push(g.next_id());
                }
                local
            }));
        }
        let mut all = HashSet::new();
        for h in handles {
            for id in h.join().unwrap() {
                assert!(all.insert(id), "duplicate id under thread contention");
            }
        }
        assert_eq!(all.len(), 8 * 2000);
    }

    #[test]
    fn custom_epoch_round_trip() {
        let epoch = 1_700_000_000_000_u64;
        let gen = Snowflake::with_epoch(9, epoch);
        let id = gen.next_id();
        let (ts_offset, worker, _) = Snowflake::parts(id);
        assert_eq!(worker, 9);
        assert_eq!(gen.epoch_ms(), epoch);
        let wall = ts_offset + epoch;
        assert!(wall > epoch);
    }

    #[test]
    fn parts_extracts_each_field() {
        // Construct an ID with known fields and decompose it.
        let ts: u64 = 12_345;
        let worker: u64 = 700;
        let seq: u64 = 4000;
        let id = (ts << TIMESTAMP_SHIFT) | (worker << WORKER_SHIFT) | seq;
        let (got_ts, got_w, got_s) = Snowflake::parts(id);
        assert_eq!(got_ts, ts);
        assert_eq!(got_w as u64, worker);
        assert_eq!(got_s as u64, seq);
    }
}
