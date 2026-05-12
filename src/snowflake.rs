//! # Snowflake ID generation
//!
//! Twitter Snowflake-style 64-bit IDs: 41-bit timestamp + 10-bit
//! worker ID + 12-bit sequence number. Distributed-safe when each
//! worker gets a unique ID. Monotonic within a worker.
//!
//! In `0.1.0` this is a placeholder implementation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default epoch (2026-01-01T00:00:00Z in milliseconds since UNIX epoch).
pub const DEFAULT_EPOCH_MS: u64 = 1_767_225_600_000;

/// Snowflake ID generator.
///
/// Holds the worker ID and the running sequence counter.
///
/// # Example
///
/// ```
/// use id_forge::snowflake::Snowflake;
///
/// let mut gen = Snowflake::new(1);
/// let id = gen.next_id();
/// ```
#[derive(Debug)]
pub struct Snowflake {
    worker_id: u16,
    epoch_ms: u64,
    sequence: AtomicU64,
}

impl Snowflake {
    /// Build a new generator with the given worker ID (0-1023) and
    /// the default epoch.
    pub fn new(worker_id: u16) -> Self {
        Self::with_epoch(worker_id, DEFAULT_EPOCH_MS)
    }

    /// Build a new generator with a custom epoch.
    pub fn with_epoch(worker_id: u16, epoch_ms: u64) -> Self {
        Self {
            worker_id: worker_id & 0x3ff,
            epoch_ms,
            sequence: AtomicU64::new(0),
        }
    }

    /// Generate the next ID.
    ///
    /// In `0.1.0` this is a placeholder. The real per-millisecond
    /// sequence-rollover logic lands in `0.9.x`.
    pub fn next_id(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let ts = now.saturating_sub(self.epoch_ms) & 0x1ffffffffff; // 41 bits
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed) & 0xfff; // 12 bits
        let worker = self.worker_id as u64 & 0x3ff; // 10 bits

        (ts << 22) | (worker << 12) | seq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_id_produces_value() {
        let gen = Snowflake::new(1);
        let _ = gen.next_id();
    }

    #[test]
    fn worker_id_clamped() {
        let gen = Snowflake::new(0xffff);
        let id = gen.next_id();
        let worker = (id >> 12) & 0x3ff;
        assert_eq!(worker, 0x3ff);
    }

    #[test]
    fn unique_ids() {
        let gen = Snowflake::new(1);
        let a = gen.next_id();
        let b = gen.next_id();
        assert_ne!(a, b);
    }

    #[test]
    fn monotonic_within_ms() {
        let gen = Snowflake::new(1);
        let a = gen.next_id();
        let b = gen.next_id();
        assert!(b >= a);
    }
}
