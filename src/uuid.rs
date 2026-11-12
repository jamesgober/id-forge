//! # UUID generation
//!
//! UUID v4 (random) and v7 (time-ordered) per RFC 9562.
//!
//! UUIDs produced by this module set the version and variant bits exactly
//! as the RFC requires, so they round-trip through any conformant parser.
//!
//! ```
//! use id_forge::uuid::Uuid;
//!
//! let v4 = Uuid::v4();
//! let v7 = Uuid::v7();
//! let parsed = Uuid::parse_str(&v4.to_string()).unwrap();
//! assert_eq!(v4, parsed);
//! assert_eq!(v7.to_string().len(), 36);
//! ```
//!
//! ## Randomness
//!
//! The random portion comes from the operating system's random source
//! (through `mod-rand`'s `tier3`), read in blocks into a per-thread
//! buffer so most IDs cost no system call. v4 UUIDs are unguessable,
//! as RFC 9562 expects. v7 UUIDs also carry their creation time.
//!
//! v7 UUIDs from different milliseconds sort in time order. UUIDs
//! minted in the same millisecond sort in random order, because v7
//! has no counter field. Use a ULID (ordered per thread) or a
//! shared Snowflake generator (ordered across threads) when IDs
//! minted in the same millisecond must sort in issue order.
//!
//! ## Errors
//!
//! [`Uuid::v4`] and [`Uuid::v7`] panic if the OS random source cannot
//! be read; they never fall back to a weaker generator.
//! [`Uuid::try_v4`] and [`Uuid::try_v7`] return a
//! [`RandomError`] instead.
//!
//! ## Processes and snapshots
//!
//! A child created with `fork()` does not reuse the parent's random
//! buffer: the buffer is discarded in the child before its next ID, so
//! parent and child never hand out the same random bytes. A restored
//! VM or container snapshot cannot be detected, so a snapshot that is
//! restored more than once can repeat IDs minted after the restore.
//! Restore a snapshot once, or start a fresh process after each
//! restore before minting IDs.
//!
//! ## Using IDs as secrets
//!
//! A v4 UUID is hard to guess, but treat it as a secret only with
//! care. `==` on [`Uuid`] stops at the first differing byte, so
//! compare a presented secret in constant time (for example with the
//! `crypt-io` crate), or store and look up a hash of it. `Debug` and
//! `Display` print the full value, so keep secret IDs out of logs.

use core::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::rng::{self, RandomError};

/// A 128-bit UUID.
///
/// The internal representation is 16 big-endian bytes per RFC 9562.
///
/// # Example
///
/// ```
/// use id_forge::uuid::Uuid;
///
/// let id = Uuid::v4();
/// assert_eq!(id.to_string().len(), 36);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Uuid([u8; 16]);

impl Uuid {
    /// The Nil UUID: all 128 bits set to zero (RFC 9562 §5.9).
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// assert_eq!(Uuid::nil().to_string(), "00000000-0000-0000-0000-000000000000");
    /// ```
    pub const fn nil() -> Self {
        Self([0u8; 16])
    }

    /// The Max UUID: all 128 bits set to one (RFC 9562 §5.10).
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// assert_eq!(Uuid::max().to_string(), "ffffffff-ffff-ffff-ffff-ffffffffffff");
    /// ```
    pub const fn max() -> Self {
        Self([0xff; 16])
    }

    /// Construct a v4 (random) UUID per RFC 9562 §5.4.
    ///
    /// 122 random bits with the version nibble set to `0100` and the
    /// variant bits set to `10` (RFC 4122 layout).
    ///
    /// # Panics
    ///
    /// Panics if the OS random source fails. Use [`Uuid::try_v4`] to
    /// get an error instead.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// let id = Uuid::v4();
    /// assert_eq!(id.version(), 4);
    /// ```
    pub fn v4() -> Self {
        match Self::try_v4() {
            Ok(id) => id,
            Err(e) => rng::fail(e),
        }
    }

    /// Fallible form of [`Uuid::v4`].
    ///
    /// Returns [`RandomError`] instead of panicking when the OS random
    /// source fails.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// let id = Uuid::try_v4().expect("OS random source");
    /// assert_eq!(id.version(), 4);
    /// ```
    pub fn try_v4() -> Result<Self, RandomError> {
        let mut bytes = rng::try_next_bytes_16()?;
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Ok(Self(bytes))
    }

    /// Construct a v7 (time-ordered) UUID per RFC 9562 §5.7.
    ///
    /// 48-bit big-endian millisecond timestamp prefix, 74 random bits,
    /// with the version nibble set to `0111` and the RFC 4122 variant bits.
    /// Two v7 IDs generated in different milliseconds compare in
    /// timestamp order byte-wise. IDs from the same millisecond are in
    /// random order.
    ///
    /// # Panics
    ///
    /// Panics if the OS random source fails. Use [`Uuid::try_v7`] to
    /// get an error instead.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// let id = Uuid::v7();
    /// assert_eq!(id.version(), 7);
    /// ```
    pub fn v7() -> Self {
        match Self::try_v7() {
            Ok(id) => id,
            Err(e) => rng::fail(e),
        }
    }

    /// Fallible form of [`Uuid::v7`].
    ///
    /// Returns [`RandomError`] instead of panicking when the OS random
    /// source fails.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// let id = Uuid::try_v7().expect("OS random source");
    /// assert_eq!(id.version(), 7);
    /// ```
    pub fn try_v7() -> Result<Self, RandomError> {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let mut bytes = rng::try_next_bytes_16()?;
        let ms_bytes = ms.to_be_bytes();
        bytes[0..6].copy_from_slice(&ms_bytes[2..8]);
        bytes[6] = (bytes[6] & 0x0f) | 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Ok(Self(bytes))
    }

    /// Wrap a 16-byte big-endian representation.
    ///
    /// The bytes are taken as-is; no version or variant bits are
    /// touched. Use this to round-trip an externally generated UUID
    /// or to reconstruct one from storage.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// let id = Uuid::v4();
    /// let copy = Uuid::from_bytes(id.as_bytes());
    /// assert_eq!(id, copy);
    /// ```
    pub const fn from_bytes(bytes: &[u8; 16]) -> Self {
        Self(*bytes)
    }

    /// Return the raw 16-byte big-endian representation.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// let id = Uuid::nil();
    /// assert_eq!(id.as_bytes(), &[0u8; 16]);
    /// ```
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Return the version nibble (the high 4 bits of byte 6).
    ///
    /// `4` for v4, `7` for v7, `0` for [`Uuid::nil`], `15` for [`Uuid::max`].
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// assert_eq!(Uuid::v4().version(), 4);
    /// assert_eq!(Uuid::v7().version(), 7);
    /// assert_eq!(Uuid::nil().version(), 0);
    /// ```
    pub const fn version(&self) -> u8 {
        self.0[6] >> 4
    }

    /// Parse a UUID from its canonical 36-character hyphenated form
    /// (e.g. `f47ac10b-58cc-4372-a567-0e02b2c3d479`).
    ///
    /// Parsing is case-insensitive. Returns [`ParseError`] if the
    /// input is not exactly 36 characters, has hyphens in the wrong
    /// positions, or contains a non-hex digit.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// let id = Uuid::parse_str("f47ac10b-58cc-4372-a567-0e02b2c3d479").unwrap();
    /// assert_eq!(id.to_string(), "f47ac10b-58cc-4372-a567-0e02b2c3d479");
    /// ```
    pub fn parse_str(input: &str) -> Result<Self, ParseError> {
        let bytes = input.as_bytes();
        if bytes.len() != 36 {
            return Err(ParseError::InvalidLength(bytes.len()));
        }
        let hyphen_positions = [8usize, 13, 18, 23];
        for &p in &hyphen_positions {
            if bytes[p] != b'-' {
                return Err(ParseError::InvalidGroup(p));
            }
        }
        let mut out = [0u8; 16];
        let mut hex_idx = 0;
        let mut byte_idx = 0;
        while hex_idx < 36 {
            if hyphen_positions.contains(&hex_idx) {
                hex_idx += 1;
                continue;
            }
            let hi = hex_value(bytes[hex_idx]).ok_or(ParseError::InvalidChar(hex_idx))?;
            let lo = hex_value(bytes[hex_idx + 1]).ok_or(ParseError::InvalidChar(hex_idx + 1))?;
            out[byte_idx] = (hi << 4) | lo;
            byte_idx += 1;
            hex_idx += 2;
        }
        Ok(Self(out))
    }
}

impl Default for Uuid {
    fn default() -> Self {
        Self::nil()
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = [0u8; 36];
        let mut o = 0;
        for (i, &byte) in self.0.iter().enumerate() {
            if matches!(i, 4 | 6 | 8 | 10) {
                out[o] = b'-';
                o += 1;
            }
            out[o] = HEX[(byte >> 4) as usize];
            out[o + 1] = HEX[(byte & 0x0f) as usize];
            o += 2;
        }
        // Every byte written above is ASCII.
        f.write_str(core::str::from_utf8(&out).unwrap_or(""))
    }
}

impl core::str::FromStr for Uuid {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

/// Error returned by [`Uuid::parse_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// Input was not exactly 36 characters. The value is the actual length.
    InvalidLength(usize),
    /// A hyphen was missing at the given byte position (expected 8, 13, 18, or 23).
    InvalidGroup(usize),
    /// A non-hex digit was found at the given byte position.
    InvalidChar(usize),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(n) => write!(f, "expected 36 characters, got {n}"),
            Self::InvalidGroup(p) => write!(f, "expected hyphen at position {p}"),
            Self::InvalidChar(p) => write!(f, "invalid hex digit at position {p}"),
        }
    }
}

impl std::error::Error for ParseError {}

#[inline]
const fn hex_value(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_version_and_variant() {
        let id = Uuid::v4();
        assert_eq!(id.version(), 4);
        assert_eq!(id.0[8] & 0xc0, 0x80);
    }

    #[test]
    fn v7_version_and_variant() {
        let id = Uuid::v7();
        assert_eq!(id.version(), 7);
        assert_eq!(id.0[8] & 0xc0, 0x80);
    }

    #[test]
    fn display_format_canonical() {
        let id = Uuid::v4();
        let s = id.to_string();
        assert_eq!(s.len(), 36);
        let hyphen_positions: Vec<usize> = s
            .char_indices()
            .filter_map(|(i, c)| if c == '-' { Some(i) } else { None })
            .collect();
        assert_eq!(hyphen_positions, vec![8, 13, 18, 23]);
    }

    #[test]
    fn v4_pair_differs() {
        assert_ne!(Uuid::v4(), Uuid::v4());
    }

    #[test]
    fn v7_pair_differs() {
        assert_ne!(Uuid::v7(), Uuid::v7());
    }

    #[test]
    fn v7_time_ordered_across_ms() {
        let a = Uuid::v7();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = Uuid::v7();
        assert!(b.as_bytes() > a.as_bytes());
    }

    #[test]
    fn nil_and_max() {
        assert_eq!(Uuid::nil().as_bytes(), &[0u8; 16]);
        assert_eq!(Uuid::max().as_bytes(), &[0xffu8; 16]);
        assert_eq!(
            Uuid::nil().to_string(),
            "00000000-0000-0000-0000-000000000000"
        );
        assert_eq!(
            Uuid::max().to_string(),
            "ffffffff-ffff-ffff-ffff-ffffffffffff"
        );
    }

    #[test]
    fn default_is_nil() {
        assert_eq!(Uuid::default(), Uuid::nil());
    }

    #[test]
    fn from_bytes_roundtrip() {
        let id = Uuid::v4();
        assert_eq!(Uuid::from_bytes(id.as_bytes()), id);
    }

    // RFC 9562 Appendix A.2 — v4 example.
    #[test]
    fn parse_rfc9562_v4_example() {
        let s = "919108f7-52d1-4320-9bac-f847db4148a8";
        let id = Uuid::parse_str(s).unwrap();
        assert_eq!(id.version(), 4);
        assert_eq!(id.0[8] & 0xc0, 0x80);
        assert_eq!(id.to_string(), s);
    }

    // RFC 9562 Appendix A.6 — v7 example.
    #[test]
    fn parse_rfc9562_v7_example() {
        let s = "017f22e2-79b0-7cc3-98c4-dc0c0c07398f";
        let id = Uuid::parse_str(s).unwrap();
        assert_eq!(id.version(), 7);
        assert_eq!(id.0[8] & 0xc0, 0x80);
        assert_eq!(id.to_string(), s);
    }

    #[test]
    fn parse_uppercase() {
        let id = Uuid::parse_str("F47AC10B-58CC-4372-A567-0E02B2C3D479").unwrap();
        assert_eq!(id.to_string(), "f47ac10b-58cc-4372-a567-0e02b2c3d479");
    }

    #[test]
    fn parse_nil() {
        let id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        assert_eq!(id, Uuid::nil());
    }

    #[test]
    fn parse_rejects_short() {
        assert!(matches!(
            Uuid::parse_str("abc"),
            Err(ParseError::InvalidLength(3))
        ));
    }

    #[test]
    fn parse_rejects_missing_hyphen() {
        assert!(matches!(
            Uuid::parse_str("f47ac10b_58cc-4372-a567-0e02b2c3d479"),
            Err(ParseError::InvalidGroup(8))
        ));
    }

    #[test]
    fn parse_rejects_bad_hex() {
        assert!(matches!(
            Uuid::parse_str("g47ac10b-58cc-4372-a567-0e02b2c3d479"),
            Err(ParseError::InvalidChar(0))
        ));
    }

    #[test]
    fn from_str_works() {
        let id: Uuid = "f47ac10b-58cc-4372-a567-0e02b2c3d479".parse().unwrap();
        assert_eq!(id.to_string(), "f47ac10b-58cc-4372-a567-0e02b2c3d479");
    }

    #[test]
    fn display_matches_reference_formatting() {
        fn reference(b: &[u8; 16]) -> String {
            format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
            )
        }
        for _ in 0..1000 {
            let id = Uuid::v4();
            assert_eq!(id.to_string(), reference(id.as_bytes()));
        }
        let mut seq = [0u8; 16];
        for (i, b) in seq.iter_mut().enumerate() {
            *b = (i as u8) * 17;
        }
        let id = Uuid::from_bytes(&seq);
        assert_eq!(id.to_string(), reference(&seq));
    }

    #[test]
    fn try_v4_and_try_v7_report_random_failure() {
        rng::test_hooks::set_fail(true);
        let a = Uuid::try_v4();
        let b = Uuid::try_v7();
        rng::test_hooks::set_fail(false);
        assert!(a.is_err());
        assert!(b.is_err());
        assert_eq!(Uuid::try_v4().unwrap().version(), 4);
        assert_eq!(Uuid::try_v7().unwrap().version(), 7);
    }

    #[test]
    #[should_panic(expected = "OS random source failed")]
    fn v4_panics_on_random_failure() {
        rng::test_hooks::set_fail(true);
        let _ = Uuid::v4();
    }

    #[test]
    fn many_v4_unique() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for _ in 0..10_000 {
            assert!(set.insert(Uuid::v4()));
        }
    }
}
