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
//! The random portion is filled by an inline xoshiro256\*\* generator
//! seeded from process ID, wall-clock nanoseconds, and a per-process
//! counter. This is fast and unpredictable across processes, but it is
//! **not** cryptographically secure — use a CSPRNG for session tokens
//! or API keys.

use core::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::rng;

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
    /// # Example
    ///
    /// ```
    /// use id_forge::uuid::Uuid;
    ///
    /// let id = Uuid::v4();
    /// assert_eq!(id.version(), 4);
    /// ```
    pub fn v4() -> Self {
        let mut bytes = rng::next_bytes_16();
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(bytes)
    }

    /// Construct a v7 (time-ordered) UUID per RFC 9562 §5.7.
    ///
    /// 48-bit big-endian millisecond timestamp prefix, 74 random bits,
    /// with the version nibble set to `0111` and the RFC 4122 variant bits.
    /// Two v7 IDs generated in different milliseconds compare in
    /// timestamp order byte-wise.
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
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let mut bytes = rng::next_bytes_16();
        let ms_bytes = ms.to_be_bytes();
        bytes[0..6].copy_from_slice(&ms_bytes[2..8]);
        bytes[6] = (bytes[6] & 0x0f) | 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(bytes)
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
        let b = &self.0;
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
            b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
        )
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
    fn many_v4_unique() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for _ in 0..10_000 {
            assert!(set.insert(Uuid::v4()));
        }
    }
}
