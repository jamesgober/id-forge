//! # NanoID generation
//!
//! URL-safe random strings of configurable length over a configurable
//! alphabet. The default alphabet is the 64-character URL-safe set
//! (`A-Z`, `a-z`, `0-9`, `_`, `-`) and the default length is 21 —
//! the same defaults as the original JavaScript reference, giving
//! roughly 1 trillion years to a 1 % collision probability at 1000
//! IDs/second.
//!
//! ```
//! use id_forge::nanoid;
//!
//! let id = nanoid::generate();
//! assert_eq!(id.len(), 21);
//!
//! let short = nanoid::with_length(8);
//! assert_eq!(short.len(), 8);
//!
//! let hex = nanoid::custom(16, b"0123456789abcdef");
//! assert_eq!(hex.len(), 16);
//! assert!(hex.chars().all(|c| "0123456789abcdef".contains(c)));
//! ```
//!
//! ## Bias-free selection
//!
//! Each character is picked from OS random bits by mask and reject:
//! the bits are masked to the smallest power of two that covers the
//! alphabet, and indices at or above the alphabet's size are thrown
//! away. For a 64-character alphabet every draw is used; for a
//! 17-character alphabet 17 of 32 are. Either way, every character of
//! the alphabet is equally likely.
//!
//! ## Alphabets
//!
//! An alphabet is a byte slice and every byte is one character, so use
//! ASCII alphabets. [`custom`] and [`try_custom`] map each byte to the
//! character with the same code point, which turns a multi-byte UTF-8
//! alphabet into different text. [`try_with_alphabet`] rejects any
//! byte that is not ASCII.
//!
//! ## Randomness quality
//!
//! The bits come from the operating system's random source, through
//! the same per-thread buffer as `uuid` and `ulid`. A default 21-character
//! NanoID carries 126 random bits and is suitable for URL tokens and
//! other unguessable references.
//!
//! When a NanoID is used as a secret (a reset token, a session key),
//! compare a presented value in constant time (for example with the
//! `crypt-io` crate) or store and look up a hash of it, because `==`
//! on a `String` stops at the first differing byte. Keep secret IDs
//! out of logs.
//!
//! ## Processes and snapshots
//!
//! A child created with `fork()` does not reuse the parent's random
//! buffer: the buffer is discarded in the child before its next ID. A
//! restored VM or container snapshot cannot be detected, so a snapshot
//! that is restored more than once can repeat IDs minted after the
//! restore. Restore a snapshot once, or start a fresh process after
//! each restore before minting IDs.
//!
//! ## Errors
//!
//! [`generate`], [`with_length`], [`custom`] and [`try_custom`] panic
//! if the OS random source fails. [`try_generate`],
//! [`try_with_length`] and [`try_with_alphabet`] return an error
//! instead.

use core::fmt;

use crate::rng::{self, RandomError};

/// Default alphabet: URL-safe, 64 characters (`_-` + alphanumeric).
pub const DEFAULT_ALPHABET: &[u8] =
    b"_-0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Default length: 21 characters.
pub const DEFAULT_LENGTH: usize = 21;

/// Generate a NanoID using the default alphabet and length.
///
/// # Panics
///
/// Panics if the OS random source fails. Use [`try_generate`] to get
/// an error instead.
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

/// Fallible form of [`generate`].
///
/// Returns [`RandomError`] instead of panicking when the OS random
/// source fails.
///
/// # Example
///
/// ```
/// use id_forge::nanoid;
///
/// let id = nanoid::try_generate().expect("OS random source");
/// assert_eq!(id.len(), 21);
/// ```
pub fn try_generate() -> Result<String, RandomError> {
    try_custom_unchecked(DEFAULT_LENGTH, DEFAULT_ALPHABET)
}

/// Generate a NanoID of the given length using the default alphabet.
///
/// # Panics
///
/// Panics if the OS random source fails. Use [`try_with_length`] to
/// get an error instead.
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

/// Fallible form of [`with_length`].
///
/// Returns [`RandomError`] instead of panicking when the OS random
/// source fails.
///
/// # Example
///
/// ```
/// use id_forge::nanoid;
///
/// let id = nanoid::try_with_length(10).expect("OS random source");
/// assert_eq!(id.len(), 10);
/// ```
pub fn try_with_length(length: usize) -> Result<String, RandomError> {
    if length == 0 {
        return Ok(String::new());
    }
    try_custom_unchecked(length, DEFAULT_ALPHABET)
}

/// Generate a NanoID with a custom alphabet, checking everything.
///
/// The fallible counterpart to [`custom`] and [`try_custom`]. It
/// returns an error, and never panics, when:
///
/// - the alphabet is empty or has a duplicate byte
///   ([`NanoidError::Alphabet`]),
/// - the alphabet has a byte that is not ASCII
///   ([`NanoidError::NonAscii`]),
/// - the OS random source fails ([`NanoidError::Random`]).
///
/// # Example
///
/// ```
/// use id_forge::nanoid::{self, AlphabetError, NanoidError};
///
/// let id = nanoid::try_with_alphabet(12, b"0123456789abcdef").unwrap();
/// assert_eq!(id.len(), 12);
///
/// assert!(matches!(
///     nanoid::try_with_alphabet(8, b"aab"),
///     Err(NanoidError::Alphabet(AlphabetError::Duplicate(b'a')))
/// ));
/// assert!(matches!(
///     nanoid::try_with_alphabet(8, "\u{e9}a".as_bytes()),
///     Err(NanoidError::NonAscii(0xc3))
/// ));
/// ```
pub fn try_with_alphabet(length: usize, alphabet: &[u8]) -> Result<String, NanoidError> {
    validate_alphabet(alphabet).map_err(NanoidError::Alphabet)?;
    if let Some(&b) = alphabet.iter().find(|b| !b.is_ascii()) {
        return Err(NanoidError::NonAscii(b));
    }
    if length == 0 {
        return Ok(String::new());
    }
    try_custom_unchecked(length, alphabet).map_err(NanoidError::Random)
}

/// Generate a NanoID with a custom length and alphabet.
///
/// This entry point is permissive: an empty alphabet returns the
/// empty string, and duplicate bytes in the alphabet are tolerated
/// (the repeated characters appear with higher probability). Use
/// [`try_custom`] when you want validation to surface those cases as
/// an error.
///
/// Use an ASCII alphabet: each byte becomes one character, so the
/// bytes of a multi-byte UTF-8 character come out as other
/// characters. [`try_with_alphabet`] rejects non-ASCII bytes.
///
/// # Panics
///
/// Panics if the OS random source fails. Use [`try_with_alphabet`] to
/// get an error instead.
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
    if alphabet.is_empty() || length == 0 {
        return String::new();
    }
    custom_unchecked(length, alphabet)
}

/// Strict counterpart to [`custom`]: validates the alphabet first.
///
/// Returns an error when the alphabet is empty or contains a
/// duplicate byte. A duplicate would otherwise silently skew the
/// character distribution.
///
/// Use an ASCII alphabet: each byte becomes one character.
/// [`try_with_alphabet`] also rejects non-ASCII bytes.
///
/// # Panics
///
/// Panics if the OS random source fails. Use [`try_with_alphabet`] to
/// get an error instead.
///
/// # Example
///
/// ```
/// use id_forge::nanoid::{self, AlphabetError};
///
/// let id = nanoid::try_custom(12, b"abcdef").unwrap();
/// assert_eq!(id.len(), 12);
///
/// assert_eq!(nanoid::try_custom(8, b""), Err(AlphabetError::Empty));
/// assert!(matches!(
///     nanoid::try_custom(8, b"aab"),
///     Err(AlphabetError::Duplicate(b'a'))
/// ));
/// ```
pub fn try_custom(length: usize, alphabet: &[u8]) -> Result<String, AlphabetError> {
    validate_alphabet(alphabet)?;
    if length == 0 {
        return Ok(String::new());
    }
    Ok(custom_unchecked(length, alphabet))
}

/// Verify that an alphabet is non-empty and free of duplicate bytes.
///
/// Useful for callers who want to vet a configuration value at
/// startup once instead of paying the validation cost on every
/// [`try_custom`] call.
///
/// # Example
///
/// ```
/// use id_forge::nanoid::{self, AlphabetError};
///
/// assert!(nanoid::validate_alphabet(b"01234567").is_ok());
/// assert_eq!(nanoid::validate_alphabet(b""), Err(AlphabetError::Empty));
/// assert_eq!(
///     nanoid::validate_alphabet(b"aba"),
///     Err(AlphabetError::Duplicate(b'a'))
/// );
/// ```
pub fn validate_alphabet(alphabet: &[u8]) -> Result<(), AlphabetError> {
    if alphabet.is_empty() {
        return Err(AlphabetError::Empty);
    }
    let mut seen = [false; 256];
    for &b in alphabet {
        if seen[b as usize] {
            return Err(AlphabetError::Duplicate(b));
        }
        seen[b as usize] = true;
    }
    Ok(())
}

/// Error returned by [`try_custom`] and [`validate_alphabet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphabetError {
    /// The alphabet was the empty slice.
    Empty,
    /// The given byte appears more than once. Duplicates would skew
    /// the character distribution silently.
    Duplicate(u8),
}

impl fmt::Display for AlphabetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("nanoid: alphabet must be non-empty"),
            Self::Duplicate(b) => {
                write!(f, "nanoid: duplicate byte 0x{b:02x} in alphabet")
            }
        }
    }
}

impl std::error::Error for AlphabetError {}

/// Error returned by [`try_with_alphabet`].
#[derive(Debug)]
#[non_exhaustive]
pub enum NanoidError {
    /// The alphabet is empty or has a duplicate byte.
    Alphabet(AlphabetError),
    /// The alphabet has this byte, which is not ASCII.
    NonAscii(u8),
    /// The OS random source failed.
    Random(RandomError),
}

impl fmt::Display for NanoidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Alphabet(e) => fmt::Display::fmt(e, f),
            Self::NonAscii(b) => {
                write!(f, "nanoid: alphabet byte 0x{b:02x} is not ASCII")
            }
            Self::Random(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for NanoidError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Alphabet(e) => Some(e),
            Self::NonAscii(_) => None,
            Self::Random(e) => Some(e),
        }
    }
}

fn custom_unchecked(length: usize, alphabet: &[u8]) -> String {
    match try_custom_unchecked(length, alphabet) {
        Ok(id) => id,
        Err(e) => rng::fail(e),
    }
}

fn try_custom_unchecked(length: usize, alphabet: &[u8]) -> Result<String, RandomError> {
    let n = alphabet.len();
    if n == 1 {
        // Degenerate: every character is the same.
        return Ok(core::iter::repeat(alphabet[0] as char)
            .take(length)
            .collect());
    }
    let bits = mask_bits(n);
    let mask: u64 = (1u64 << bits) - 1;

    let mut out = String::with_capacity(length);
    let mut buffer: u64 = 0;
    let mut buffer_bits: u32 = 0;
    let mut placed = 0;

    while placed < length {
        if buffer_bits < bits {
            buffer = rng::try_next_u64()?;
            buffer_bits = 64;
        }
        let idx = (buffer & mask) as usize;
        buffer >>= bits;
        buffer_bits -= bits;
        if idx < n {
            out.push(alphabet[idx] as char);
            placed += 1;
        }
    }
    Ok(out)
}

/// Bits needed to address any index in `0..n` — the smallest `k`
/// such that `2^k >= n`. For `n = 64` this is 6; for `n = 65` it's 7.
#[inline]
const fn mask_bits(n: usize) -> u32 {
    // n is guaranteed > 1 here.
    usize::BITS - (n - 1).leading_zeros()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

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
        let id = custom(64, b"01");
        assert!(id.chars().all(|c| c == '0' || c == '1'));
        assert_eq!(id.len(), 64);
    }

    #[test]
    fn unique_ids() {
        assert_ne!(generate(), generate());
    }

    #[test]
    fn many_default_unique() {
        let mut set = HashSet::new();
        for _ in 0..10_000 {
            assert!(set.insert(generate()));
        }
    }

    #[test]
    fn empty_alphabet_returns_empty() {
        assert_eq!(custom(10, &[]), "");
    }

    #[test]
    fn zero_length_returns_empty() {
        assert_eq!(custom(0, DEFAULT_ALPHABET), "");
        assert_eq!(with_length(0), "");
    }

    #[test]
    fn single_char_alphabet() {
        let id = custom(8, b"x");
        assert_eq!(id, "xxxxxxxx");
    }

    #[test]
    fn try_custom_rejects_empty() {
        assert_eq!(try_custom(8, b""), Err(AlphabetError::Empty));
    }

    #[test]
    fn try_custom_rejects_duplicate() {
        let err = try_custom(8, b"abcda").unwrap_err();
        assert_eq!(err, AlphabetError::Duplicate(b'a'));
    }

    #[test]
    fn try_custom_accepts_valid() {
        let id = try_custom(12, b"abcdef0123").unwrap();
        assert_eq!(id.len(), 12);
        assert!(id.chars().all(|c| "abcdef0123".contains(c)));
    }

    #[test]
    fn validate_alphabet_paths() {
        assert!(validate_alphabet(DEFAULT_ALPHABET).is_ok());
        assert_eq!(validate_alphabet(b""), Err(AlphabetError::Empty));
        assert_eq!(
            validate_alphabet(b"abca"),
            Err(AlphabetError::Duplicate(b'a'))
        );
    }

    #[test]
    fn non_power_of_two_alphabet_unbiased() {
        // 17-char alphabet: rejection sampling must keep distribution
        // uniform. Over a large sample no single char dominates.
        let alphabet: &[u8] = b"ABCDEFGHIJKLMNOPQ"; // 17 chars
        let id = custom(170_000, alphabet);
        let mut counts = [0usize; 17];
        for c in id.bytes() {
            let i = alphabet.iter().position(|&b| b == c).unwrap();
            counts[i] += 1;
        }
        // Expected ~10 000 per char; allow 12 % skew.
        for (i, &n) in counts.iter().enumerate() {
            assert!(
                (8_800..=11_200).contains(&n),
                "alphabet[{i}] ({}) count {n} outside expected band",
                alphabet[i] as char
            );
        }
    }

    #[test]
    fn mask_bits_known_values() {
        assert_eq!(mask_bits(2), 1);
        assert_eq!(mask_bits(8), 3);
        assert_eq!(mask_bits(64), 6);
        assert_eq!(mask_bits(65), 7);
        assert_eq!(mask_bits(256), 8);
    }

    #[test]
    fn length_exact_across_alphabet_sizes() {
        // ASCII-printable alphabets at increasing sizes to exercise
        // every mask-bits path (1 .. 8). Output char count must equal
        // the requested length even when acceptance rate is below 100 %.
        let printable: Vec<u8> = (b'!'..=b'~').collect(); // 94 unique ASCII bytes
        for n in [2usize, 7, 16, 33, 64, 65, 93, 94] {
            let alphabet = &printable[..n];
            let id = custom(50, alphabet);
            assert_eq!(id.chars().count(), 50, "size {n}");
            assert_eq!(id.len(), 50, "size {n}: ASCII alphabet so bytes==chars");
        }
    }

    #[test]
    fn non_ascii_alphabet_counts_chars_not_bytes() {
        // 200 unique bytes including non-ASCII. Output must have
        // `length` characters even though each char may encode to
        // multiple UTF-8 bytes in the returned String.
        let printable: Vec<u8> = (0..=255).collect();
        let id = custom(30, &printable);
        assert_eq!(id.chars().count(), 30);
    }

    #[test]
    fn try_with_alphabet_rejects_non_ascii() {
        assert!(matches!(
            try_with_alphabet(16, "\u{e9}".as_bytes()),
            Err(NanoidError::NonAscii(0xc3))
        ));
        assert!(matches!(
            try_with_alphabet(16, "\u{65e5}\u{672c}".as_bytes()),
            Err(NanoidError::Alphabet(AlphabetError::Duplicate(_))) | Err(NanoidError::NonAscii(_))
        ));
        let id = try_with_alphabet(16, b"abc").unwrap();
        assert_eq!(id.len(), 16);
        assert!(id.bytes().all(|b| b"abc".contains(&b)));
        assert_eq!(try_with_alphabet(0, b"abc").unwrap(), "");
    }

    #[test]
    fn try_variants_report_random_failure() {
        rng::test_hooks::set_fail(true);
        let a = try_generate();
        let b = try_with_length(30);
        let c = try_with_alphabet(30, b"0123456789");
        rng::test_hooks::set_fail(false);
        assert!(a.is_err());
        assert!(b.is_err());
        assert!(matches!(c, Err(NanoidError::Random(_))));
        assert_eq!(try_generate().unwrap().len(), DEFAULT_LENGTH);
        assert_eq!(try_with_length(30).unwrap().len(), 30);
    }

    #[test]
    fn default_alphabet_has_no_duplicates() {
        assert!(validate_alphabet(DEFAULT_ALPHABET).is_ok());
        assert_eq!(DEFAULT_ALPHABET.len(), 64);
    }
}
