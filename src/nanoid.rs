//! # NanoID generation
//!
//! URL-safe random strings of configurable length using a
//! configurable alphabet. Default: 21-character A-Z, a-z, 0-9, _, -.
//!
//! In `0.1.0` this is a placeholder implementation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default alphabet: URL-safe, 64 characters.
pub const DEFAULT_ALPHABET: &[u8] =
    b"_-0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Default length: 21 characters (1 trillion years to collision at 1k/sec).
pub const DEFAULT_LENGTH: usize = 21;

/// Generate a NanoID using the default alphabet and length.
///
/// # Example
///
/// ```
/// use id_forge::nanoid;
///
/// let id = nanoid::generate();
/// assert_eq!(id.len(), 21);
/// ```
pub fn generate() -> String {
    custom(DEFAULT_LENGTH, DEFAULT_ALPHABET)
}

/// Generate a NanoID of the given length.
///
/// # Example
///
/// ```
/// use id_forge::nanoid;
///
/// let id = nanoid::with_length(10);
/// assert_eq!(id.len(), 10);
/// ```
pub fn with_length(length: usize) -> String {
    custom(length, DEFAULT_ALPHABET)
}

/// Generate a NanoID with a custom length and alphabet.
///
/// # Example
///
/// ```
/// use id_forge::nanoid;
///
/// let id = nanoid::custom(8, b"0123456789ABCDEF");
/// assert_eq!(id.len(), 8);
/// assert!(id.chars().all(|c| "0123456789ABCDEF".contains(c)));
/// ```
pub fn custom(length: usize, alphabet: &[u8]) -> String {
    if alphabet.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(length);
    let mut state = placeholder_state();
    let n = alphabet.len() as u64;
    for _ in 0..length {
        state = state
            .wrapping_mul(0x9E3779B97F4A7C15)
            .wrapping_add(0xBF58476D1CE4E5B9);
        let idx = (state >> 33) % n;
        out.push(alphabet[idx as usize] as char);
    }
    out
}

fn placeholder_state() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    nanos ^ counter.wrapping_mul(0x94D049BB133111EB)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_length() {
        assert_eq!(generate().len(), 21);
    }

    #[test]
    fn with_length_correct() {
        assert_eq!(with_length(10).len(), 10);
    }

    #[test]
    fn custom_alphabet_respected() {
        let id = custom(20, b"01");
        assert!(id.chars().all(|c| c == '0' || c == '1'));
    }

    #[test]
    fn unique_ids() {
        let a = generate();
        let b = generate();
        assert_ne!(a, b);
    }
}
