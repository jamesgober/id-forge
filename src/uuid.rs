//! # UUID generation
//!
//! UUID v4 (random) and v7 (time-ordered) per RFC 9562.
//!
//! In `0.1.0` these are placeholder implementations.

use core::fmt;

/// A 128-bit UUID.
///
/// # Example
///
/// ```
/// use id_forge::uuid::Uuid;
///
/// let id = Uuid::v4();
/// assert_eq!(id.to_string().len(), 36);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Uuid([u8; 16]);

impl Uuid {
    /// Construct a v4 (random) UUID.
    ///
    /// In `0.1.0` this is a placeholder using a deterministic source.
    /// The real implementation lands in `0.9.x`.
    pub fn v4() -> Self {
        let mut bytes = placeholder_bytes();
        // Set version (4) and variant (RFC 4122) bits.
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(bytes)
    }

    /// Construct a v7 (time-ordered) UUID.
    ///
    /// In `0.1.0` this is a placeholder. Real time-based prefix
    /// lands in `0.9.x`.
    pub fn v7() -> Self {
        let mut bytes = placeholder_bytes();
        bytes[6] = (bytes[6] & 0x0f) | 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(bytes)
    }

    /// Return the raw 16-byte representation.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = &self.0;
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
            b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
        )
    }
}

fn placeholder_bytes() -> [u8; 16] {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut out = [0u8; 16];
    out[0..8].copy_from_slice(&nanos.to_be_bytes());
    out[8..16].copy_from_slice(&counter.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_has_correct_version() {
        let id = Uuid::v4();
        assert_eq!(id.0[6] & 0xf0, 0x40);
    }

    #[test]
    fn v7_has_correct_version() {
        let id = Uuid::v7();
        assert_eq!(id.0[6] & 0xf0, 0x70);
    }

    #[test]
    fn display_format_correct() {
        let id = Uuid::v4();
        let s = id.to_string();
        assert_eq!(s.len(), 36);
        assert_eq!(s.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn unique() {
        let a = Uuid::v4();
        let b = Uuid::v4();
        assert_ne!(a, b);
    }
}
