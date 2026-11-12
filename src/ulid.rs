//! # ULID generation
//!
//! Universally Unique Lexicographically Sortable Identifier per the
//! [ULID spec]: 128 bits split as a 48-bit big-endian millisecond
//! timestamp followed by 80 bits of randomness. Encoded as 26
//! Crockford base32 characters that sort byte-wise in creation order.
//!
//! [ULID spec]: https://github.com/ulid/spec
//!
//! ```
//! use id_forge::ulid::Ulid;
//!
//! let a = Ulid::new();
//! let b = Ulid::new();
//! assert_eq!(a.to_string().len(), 26);
//! assert!(b > a);                                  // monotonic per thread
//! let parsed = Ulid::parse_str(&a.to_string()).unwrap();
//! assert_eq!(a, parsed);
//! ```
//!
//! ## Monotonicity
//!
//! On one thread, every ULID is strictly greater than the one before
//! it. Two ULIDs generated in the same millisecond on the same thread
//! are byte-wise ordered: the second one is the first one's 80-bit
//! random suffix plus one. If the wall clock steps back (an NTP
//! correction, a VM migration), the thread keeps the last timestamp it
//! used and keeps counting up from it, so ordering still holds. Across
//! milliseconds, fresh randomness is drawn. This is the spec's
//! "monotonic factory", with one factory per thread.
//!
//! The guarantee does not extend across threads. Two threads that
//! mint ULIDs in the same millisecond each start from their own random
//! suffix, so a ULID from one thread can sort before a ULID that
//! another thread minted earlier. When IDs from several threads must
//! sort in issue order, share one Snowflake generator
//! (`id_forge::snowflake::Snowflake`), which is monotonic across
//! threads.
//!
//! ## Randomness
//!
//! The 80-bit random suffix comes from the operating system's random
//! source, through the same per-thread buffer as `uuid`. A ULID also
//! carries its creation time, and IDs minted in the same millisecond
//! on one thread differ by one, so treat a ULID as an identifier, not
//! a secret.
//!
//! ## Processes and snapshots
//!
//! A child created with `fork()` does not reuse the parent's random
//! buffer or its monotonic state: both are reset in the child before
//! its next ULID. A restored VM or container snapshot cannot be
//! detected, so a snapshot that is restored more than once can repeat
//! ULIDs (and every other random ID) minted after the restore. Restore
//! a snapshot once, or start a fresh process after each restore before
//! minting IDs.
//!
//! ## Errors
//!
//! [`Ulid::new`] panics if the OS random source fails.
//! [`Ulid::try_new`] returns a [`RandomError`] instead.

use core::fmt;
use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::rng::{self, RandomError};

/// Crockford base32 alphabet — the 32 characters ULID display uses.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

const RAND_MASK_80: u128 = (1u128 << 80) - 1;

/// A 128-bit ULID.
///
/// Internally stored as 16 big-endian bytes: 6 bytes of millisecond
/// timestamp followed by 10 bytes of randomness.
///
/// # Example
///
/// ```
/// use id_forge::ulid::Ulid;
///
/// let id = Ulid::new();
/// assert_eq!(id.to_string().len(), 26);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ulid([u8; 16]);

impl Ulid {
    /// The Nil ULID: all 128 bits set to zero.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::ulid::Ulid;
    ///
    /// assert_eq!(Ulid::nil().to_string(), "00000000000000000000000000");
    /// ```
    pub const fn nil() -> Self {
        Self([0u8; 16])
    }

    /// The Max ULID: all 128 bits set to one — the largest value the
    /// 26-character display can represent (`7ZZZ…` since the spec
    /// reserves the top two bits of the leading character).
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::ulid::Ulid;
    ///
    /// assert_eq!(Ulid::max().to_string(), "7ZZZZZZZZZZZZZZZZZZZZZZZZZ");
    /// ```
    pub const fn max() -> Self {
        Self([0xff; 16])
    }

    /// Construct a new ULID at the current wall-clock millisecond.
    ///
    /// Two ULIDs minted on the same thread are strictly ordered: in the
    /// same millisecond the second one is the first one's random suffix
    /// plus one. Across milliseconds, the randomness is freshly drawn.
    /// Ordering is per thread, not per process; see the
    /// [module docs](self#monotonicity).
    ///
    /// # Panics
    ///
    /// Panics if the OS random source fails. Use [`Ulid::try_new`] to
    /// get an error instead.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::ulid::Ulid;
    ///
    /// let a = Ulid::new();
    /// let b = Ulid::new();
    /// assert!(b > a);
    /// ```
    pub fn new() -> Self {
        match Self::try_new() {
            Ok(id) => id,
            Err(e) => rng::fail(e),
        }
    }

    /// Fallible form of [`Ulid::new`].
    ///
    /// Returns [`RandomError`] instead of panicking when the OS random
    /// source fails. The ordering guarantee is the same as
    /// [`Ulid::new`], and both share one per-thread state.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::ulid::Ulid;
    ///
    /// let id = Ulid::try_new().expect("OS random source");
    /// assert_eq!(id.to_string().len(), 26);
    /// ```
    pub fn try_new() -> Result<Self, RandomError> {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
            & ((1u64 << 48) - 1);
        STATE.with(|cell| {
            let mut st = cell.borrow_mut();
            let (ms, rand) = st.next(ms, rng::fork_token())?;
            Ok(Self::pack(ms, rand))
        })
    }

    /// Wrap a 16-byte big-endian representation as-is.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::ulid::Ulid;
    ///
    /// let id = Ulid::new();
    /// let copy = Ulid::from_bytes(id.as_bytes());
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
    /// use id_forge::ulid::Ulid;
    ///
    /// assert_eq!(Ulid::nil().as_bytes(), &[0u8; 16]);
    /// ```
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Return the 48-bit millisecond timestamp prefix.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::ulid::Ulid;
    ///
    /// let id = Ulid::new();
    /// assert!(id.timestamp_ms() > 0);
    /// ```
    pub const fn timestamp_ms(&self) -> u64 {
        let b = &self.0;
        ((b[0] as u64) << 40)
            | ((b[1] as u64) << 32)
            | ((b[2] as u64) << 24)
            | ((b[3] as u64) << 16)
            | ((b[4] as u64) << 8)
            | (b[5] as u64)
    }

    /// Parse a 26-character Crockford base32 ULID, case-insensitive.
    ///
    /// Accepts the substitution rules from the spec: `I`, `L` decode
    /// to `1`; `O` decodes to `0`. `U` is reserved and rejected.
    /// Returns [`ParseError`] on length, character, or leading-bits
    /// violations.
    ///
    /// # Example
    ///
    /// ```
    /// use id_forge::ulid::Ulid;
    ///
    /// let id = Ulid::parse_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap();
    /// assert_eq!(id.to_string(), "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    /// ```
    pub fn parse_str(input: &str) -> Result<Self, ParseError> {
        let bytes = input.as_bytes();
        if bytes.len() != 26 {
            return Err(ParseError::InvalidLength(bytes.len()));
        }
        // Leading character must be 0-7 — only the low 3 bits are
        // valid for a 128-bit value rendered in 130 bits.
        let first = decode_char(bytes[0]).ok_or(ParseError::InvalidChar(0))?;
        if first > 7 {
            return Err(ParseError::Overflow);
        }
        let mut n: u128 = first as u128;
        for (i, &c) in bytes.iter().enumerate().skip(1) {
            let v = decode_char(c).ok_or(ParseError::InvalidChar(i))?;
            n = (n << 5) | (v as u128);
        }
        Ok(Self(n.to_be_bytes()))
    }

    fn pack(ms: u64, rand: u128) -> Self {
        let mut bytes = [0u8; 16];
        let ms_bytes = ms.to_be_bytes();
        bytes[0..6].copy_from_slice(&ms_bytes[2..8]);
        let rand_bytes = rand.to_be_bytes();
        bytes[6..16].copy_from_slice(&rand_bytes[6..16]);
        Self(bytes)
    }
}

impl Default for Ulid {
    fn default() -> Self {
        Self::nil()
    }
}

impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = u128::from_be_bytes(self.0);
        let mut out = [0u8; 26];
        for i in (0..26).rev() {
            out[i] = ALPHABET[((n >> ((25 - i) * 5)) & 0x1f) as usize];
        }
        f.write_str(core::str::from_utf8(&out).unwrap_or(""))
    }
}

impl core::str::FromStr for Ulid {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

/// Error returned by [`Ulid::parse_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// Input was not exactly 26 characters. The value is the actual length.
    InvalidLength(usize),
    /// A character at the given byte position is not in the Crockford
    /// base32 alphabet (after applying the I/L/O/U substitution rules).
    InvalidChar(usize),
    /// The leading character encodes a value above 7, which would set
    /// bits beyond the 128-bit ULID range.
    Overflow,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(n) => write!(f, "expected 26 characters, got {n}"),
            Self::InvalidChar(p) => write!(f, "invalid Crockford base32 digit at position {p}"),
            Self::Overflow => write!(f, "leading character exceeds 128-bit range"),
        }
    }
}

impl std::error::Error for ParseError {}

#[inline]
const fn decode_char(c: u8) -> Option<u8> {
    match c {
        b'0' | b'o' | b'O' => Some(0),
        b'1' | b'i' | b'I' | b'l' | b'L' => Some(1),
        b'2'..=b'9' => Some(c - b'0'),
        b'A'..=b'H' => Some(c - b'A' + 10),
        b'J' | b'K' => Some(c - b'A' + 10 - 1),
        b'M' | b'N' => Some(c - b'A' + 10 - 2),
        b'P'..=b'T' => Some(c - b'A' + 10 - 3),
        b'V'..=b'Z' => Some(c - b'A' + 10 - 4),
        b'a'..=b'h' => Some(c - b'a' + 10),
        b'j' | b'k' => Some(c - b'a' + 10 - 1),
        b'm' | b'n' => Some(c - b'a' + 10 - 2),
        b'p'..=b't' => Some(c - b'a' + 10 - 3),
        b'v'..=b'z' => Some(c - b'a' + 10 - 4),
        _ => None,
    }
}

// -------- Monotonic factory state --------

thread_local! {
    static STATE: RefCell<MonotonicState> = const { RefCell::new(MonotonicState::new()) };
}

struct MonotonicState {
    last_ms: u64,
    last_rand: u128,
    fork_token: u64,
}

impl MonotonicState {
    const fn new() -> Self {
        Self {
            last_ms: 0,
            last_rand: 0,
            fork_token: 0,
        }
    }

    /// Return the `(ms, random suffix)` pair for the next ULID.
    ///
    /// `ms` is the wall clock. The returned timestamp is never below
    /// the last one, so a clock step back keeps the sequence ordered.
    fn next(&mut self, ms: u64, fork_token: u64) -> Result<(u64, u128), RandomError> {
        if fork_token != self.fork_token {
            // Copied from the parent by `fork()`: the parent may issue
            // `last_rand + 1` too, so start over with fresh randomness.
            self.last_ms = 0;
            self.last_rand = 0;
            self.fork_token = fork_token;
        }
        if ms <= self.last_ms && self.last_rand != 0 {
            let next = self.last_rand.wrapping_add(1) & RAND_MASK_80;
            assert!(
                next != 0,
                "ulid: 80-bit monotonic counter overflowed in a single millisecond"
            );
            self.last_rand = next;
        } else {
            let hi = rng::try_next_u64()? as u128;
            let lo = rng::try_next_u64()? as u128;
            self.last_ms = ms;
            // 80 random bits: 64 from `hi`, top 16 from `lo`.
            self.last_rand = ((hi & 0xFFFF_FFFF_FFFF_FFFF) << 16) | ((lo >> 48) & 0xFFFF);
        }
        Ok((self.last_ms, self.last_rand))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn display_length_26() {
        assert_eq!(Ulid::new().to_string().len(), 26);
    }

    #[test]
    fn two_calls_differ() {
        assert_ne!(Ulid::new(), Ulid::new());
    }

    #[test]
    fn monotonic_within_ms() {
        let a = Ulid::new();
        let b = Ulid::new();
        assert!(b > a, "ULIDs in same ms must be strictly ordered");
    }

    #[test]
    fn time_ordered_across_ms() {
        let a = Ulid::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = Ulid::new();
        assert!(b > a);
        assert!(b.timestamp_ms() > a.timestamp_ms());
    }

    #[test]
    fn nil_and_max() {
        assert_eq!(Ulid::nil().as_bytes(), &[0u8; 16]);
        assert_eq!(Ulid::max().as_bytes(), &[0xffu8; 16]);
        assert_eq!(Ulid::nil().to_string(), "00000000000000000000000000");
        assert_eq!(Ulid::max().to_string(), "7ZZZZZZZZZZZZZZZZZZZZZZZZZ");
    }

    #[test]
    fn default_is_nil() {
        assert_eq!(Ulid::default(), Ulid::nil());
    }

    #[test]
    fn parse_round_trip() {
        // Spec example.
        let s = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let id = Ulid::parse_str(s).unwrap();
        assert_eq!(id.to_string(), s);
    }

    #[test]
    fn parse_nil() {
        let id = Ulid::parse_str("00000000000000000000000000").unwrap();
        assert_eq!(id, Ulid::nil());
    }

    #[test]
    fn parse_max() {
        let id = Ulid::parse_str("7ZZZZZZZZZZZZZZZZZZZZZZZZZ").unwrap();
        assert_eq!(id, Ulid::max());
    }

    #[test]
    fn parse_lowercase() {
        let id = Ulid::parse_str("01arz3ndektsv4rrffq69g5fav").unwrap();
        assert_eq!(id.to_string(), "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    }

    #[test]
    fn parse_substitutions() {
        // I, L map to 1; O maps to 0.
        let a = Ulid::parse_str("0IIIIIIIIIIIIIIIIIIIIIIIII").unwrap();
        let b = Ulid::parse_str("0LLLLLLLLLLLLLLLLLLLLLLLLL").unwrap();
        let c = Ulid::parse_str("01111111111111111111111111").unwrap();
        assert_eq!(a, b);
        assert_eq!(a, c);
        let z = Ulid::parse_str("OO000000000000000000000000").unwrap();
        assert_eq!(z, Ulid::nil());
    }

    #[test]
    fn parse_rejects_short() {
        assert!(matches!(
            Ulid::parse_str("abc"),
            Err(ParseError::InvalidLength(3))
        ));
    }

    #[test]
    fn parse_rejects_long() {
        assert!(matches!(
            Ulid::parse_str("01ARZ3NDEKTSV4RRFFQ69G5FAVX"),
            Err(ParseError::InvalidLength(27))
        ));
    }

    #[test]
    fn parse_rejects_u() {
        // U is reserved.
        assert!(matches!(
            Ulid::parse_str("0UARZ3NDEKTSV4RRFFQ69G5FAV"),
            Err(ParseError::InvalidChar(1))
        ));
    }

    #[test]
    fn parse_rejects_overflow_leading() {
        // Leading char 8..=Z would set the 129th bit.
        assert!(matches!(
            Ulid::parse_str("80000000000000000000000000"),
            Err(ParseError::Overflow)
        ));
        assert!(matches!(
            Ulid::parse_str("Z0000000000000000000000000"),
            Err(ParseError::Overflow)
        ));
    }

    #[test]
    fn from_str_works() {
        let id: Ulid = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
        assert_eq!(id.to_string(), "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    }

    #[test]
    fn from_bytes_roundtrip() {
        let id = Ulid::new();
        assert_eq!(Ulid::from_bytes(id.as_bytes()), id);
    }

    #[test]
    fn timestamp_decodes_known_prefix() {
        // "01ARZ3NDEK" -> ms 0x01563E3AB5D3 (48 bits) in Crockford base32.
        let id = Ulid::parse_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap();
        assert_eq!(id.timestamp_ms(), 0x0000_0156_3E3A_B5D3);
    }

    #[test]
    fn timestamp_from_known_bytes() {
        // Hand-packed ms = 0x0123_4567_89AB in the first 6 bytes.
        let mut bytes = [0u8; 16];
        bytes[0..6].copy_from_slice(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xAB]);
        let id = Ulid::from_bytes(&bytes);
        assert_eq!(id.timestamp_ms(), 0x0000_0123_4567_89AB);
    }

    #[test]
    fn many_unique() {
        let mut set = HashSet::new();
        for _ in 0..10_000 {
            assert!(set.insert(Ulid::new()));
        }
    }

    #[test]
    fn clock_step_back_keeps_order() {
        let mut st = MonotonicState::new();
        let token = rng::fork_token();
        let (ms_a, rand_a) = st.next(1_000_000, token).unwrap();
        // The wall clock steps back 5 ms.
        let (ms_b, rand_b) = st.next(999_995, token).unwrap();
        let a = Ulid::pack(ms_a, rand_a);
        let b = Ulid::pack(ms_b, rand_b);
        assert!(
            b > a,
            "a ULID minted after a clock step back must sort after earlier ones"
        );
        assert_eq!(ms_b, ms_a);
        // The clock moves forward again: fresh randomness, later time.
        let (ms_c, _) = st.next(1_000_001, token).unwrap();
        assert_eq!(ms_c, 1_000_001);
    }

    #[test]
    fn changed_fork_token_resets_state() {
        let token = rng::fork_token();
        let mut st = MonotonicState::new();
        let (ms, rand) = st.next(1_000_000, token).unwrap();
        // A forked child holds a copy of this state, under a token
        // that no longer matches. It must not continue the counter.
        let mut child = MonotonicState {
            last_ms: ms,
            last_rand: rand,
            fork_token: token.wrapping_add(1),
        };
        let (_, child_next) = child.next(ms, token).unwrap();
        assert_ne!(
            child_next,
            rand + 1,
            "a forked child must draw a fresh suffix"
        );
        // The parent continues its own counter.
        let (_, parent_next) = st.next(ms, token).unwrap();
        assert_eq!(parent_next, rand + 1);
    }

    #[test]
    fn try_new_reports_random_failure() {
        // A new millisecond needs fresh randomness; force the refill
        // to fail and check that the error surfaces.
        std::thread::sleep(std::time::Duration::from_millis(2));
        rng::test_hooks::set_fail(true);
        let res = Ulid::try_new();
        rng::test_hooks::set_fail(false);
        assert!(res.is_err());
        assert!(Ulid::try_new().is_ok());
    }

    #[test]
    fn monotonic_within_ms_burst() {
        // 1000 in a row, all strictly increasing.
        let mut prev = Ulid::new();
        for _ in 0..1000 {
            let cur = Ulid::new();
            assert!(cur > prev);
            prev = cur;
        }
    }
}
