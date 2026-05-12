//! # ULID generation
//!
//! Universally Unique Lexicographically Sortable Identifier.
//! 128 bits: 48-bit timestamp + 80-bit randomness. Sorts naturally
//! by creation time.
//!
//! In `0.1.0` this is a placeholder implementation.

use core::fmt;

/// A 128-bit ULID.
///
/// # Example
///
/// ```
/// use id_forge::ulid::Ulid;
///
/// let id = Ulid::new();
/// assert_eq!(id.to_string().len(), 26);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ulid([u8; 16]);

impl Ulid {
    /// Construct a new ULID with the current time.
    ///
    /// In `0.1.0` this is a placeholder. The real implementation
    /// lands in `0.9.x`.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut bytes = [0u8; 16];
        // First 6 bytes: 48-bit ms timestamp.
        let ms_bytes = ms.to_be_bytes();
        bytes[0..6].copy_from_slice(&ms_bytes[2..8]);
        // Remaining 10 bytes: placeholder "randomness" derived from counter.
        bytes[6..14].copy_from_slice(&counter.to_be_bytes());
        Self(bytes)
    }

    /// Return the raw 16-byte representation.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl Default for Ulid {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Crockford base32 encoding of 128 bits = 26 chars.
        const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        // Treat the bytes as a 128-bit big-endian integer.
        let mut n: u128 = 0;
        for &b in &self.0 {
            n = (n << 8) | (b as u128);
        }
        let mut out = [0u8; 26];
        for i in (0..26).rev() {
            out[i] = ALPHABET[(n & 31) as usize];
            n >>= 5;
        }
        f.write_str(core::str::from_utf8(&out).unwrap_or(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_length_26() {
        let id = Ulid::new();
        assert_eq!(id.to_string().len(), 26);
    }

    #[test]
    fn unique() {
        let a = Ulid::new();
        let b = Ulid::new();
        assert_ne!(a, b);
    }

    #[test]
    fn time_ordered() {
        let a = Ulid::new();
        let b = Ulid::new();
        // Same millisecond may produce a==b on timestamp prefix; counter
        // ensures b > a overall.
        assert!(b.as_bytes() >= a.as_bytes());
    }
}
