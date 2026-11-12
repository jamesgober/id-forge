//! Random bytes for the `uuid`, `ulid`, and `nanoid` modules.
//!
//! Every random bit comes from the operating system's random source
//! through `mod_rand::tier3` (`getrandom(2)` on Linux, `getentropy(3)`
//! on macOS, `BCryptGenRandom` on Windows, `/dev/urandom` on other
//! Unix systems). To keep the per-ID cost low, each thread reads
//! [`POOL_LEN`] bytes at a time into a private buffer and hands them
//! out in order. Each byte is handed out once; the buffer slot is
//! overwritten with zero when it is taken. That is a plain store, not
//! a guaranteed wipe, and the unread part of the buffer stays in
//! memory until it is used.
//!
//! # Fork safety
//!
//! A child created with `fork()` gets a copy of the parent's memory,
//! including this buffer. To stop parent and child handing out the
//! same bytes, every draw compares a fork token with the one stored in
//! the buffer and throws the buffer away when they differ:
//!
//! - On Linux, Android, Apple platforms, the BSDs, illumos and
//!   Solaris, the token is a counter bumped by a `pthread_atfork`
//!   child handler. The check is one relaxed atomic load.
//! - On other Unix systems the token is the process ID, read on every
//!   draw.
//! - Elsewhere there is no `fork()`, and the token is constant.
//!
//! A restored VM or container snapshot cannot be detected this way.
//!
//! If the OS random source fails, [`RandomError`] is returned to the
//! `try_*` generators, and the panicking generators panic. Nothing is
//! ever filled from a weaker generator.

use core::fmt;
use std::cell::RefCell;

/// Bytes read from the OS per refill: 16 v4 UUIDs or ULIDs per call.
const POOL_LEN: usize = 256;

/// The operating system's random source could not be read.
///
/// Returned by the fallible generators ([`Uuid::try_v4`],
/// [`Uuid::try_v7`], [`Ulid::try_new`], [`nanoid::try_generate`],
/// [`nanoid::try_with_length`] and [`nanoid::try_with_alphabet`]).
/// The panicking generators panic with the same message instead.
///
/// No ID is produced when this error is returned, and no partial or
/// stale random bytes are used. The next call tries the OS source
/// again, so the error clears when its cause does (for example, when
/// the process has free file descriptors again).
///
/// [`Uuid::try_v4`]: crate::uuid::Uuid::try_v4
/// [`Uuid::try_v7`]: crate::uuid::Uuid::try_v7
/// [`Ulid::try_new`]: crate::ulid::Ulid::try_new
/// [`nanoid::try_generate`]: crate::nanoid::try_generate
/// [`nanoid::try_with_length`]: crate::nanoid::try_with_length
/// [`nanoid::try_with_alphabet`]: crate::nanoid::try_with_alphabet
#[derive(Debug)]
pub struct RandomError {
    source: std::io::Error,
}

impl RandomError {
    /// The kind of the underlying I/O error.
    pub fn kind(&self) -> std::io::ErrorKind {
        self.source.kind()
    }

    /// The raw OS error code, when the OS reported one.
    pub fn raw_os_error(&self) -> Option<i32> {
        self.source.raw_os_error()
    }
}

impl fmt::Display for RandomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "id-forge: the OS random source failed: {}", self.source)
    }
}

impl std::error::Error for RandomError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Fork detection. `token()` changes in a child created by `fork()`.
mod fork {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
        target_os = "illumos",
        target_os = "solaris",
    ))]
    mod imp {
        use core::ffi::c_int;
        use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

        /// Bumped in the child after every `fork()`.
        static GENERATION: AtomicU64 = AtomicU64::new(0);

        const UNREGISTERED: u8 = 0;
        const REGISTERING: u8 = 1;
        const REGISTERED: u8 = 2;
        static STATE: AtomicU8 = AtomicU8::new(UNREGISTERED);

        extern "C" {
            fn pthread_atfork(
                prepare: Option<unsafe extern "C" fn()>,
                parent: Option<unsafe extern "C" fn()>,
                child: Option<unsafe extern "C" fn()>,
            ) -> c_int;
        }

        unsafe extern "C" fn on_fork_child() {
            // Only an atomic add: async-signal-safe, no allocation.
            GENERATION.fetch_add(1, Ordering::Relaxed);
        }

        #[cold]
        fn register() {
            // No `Once`: a child forked while another thread was
            // inside `call_once` would wait on it forever. Losers of
            // the race do not wait; the winner registers once.
            if STATE
                .compare_exchange(
                    UNREGISTERED,
                    REGISTERING,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                // SAFETY: `pthread_atfork` is called with a valid
                // function pointer that has the C ABI and takes no
                // arguments. The handler only touches an atomic.
                // Registration happens at most once per process
                // image, so the handler list does not grow.
                unsafe {
                    pthread_atfork(None, None, Some(on_fork_child));
                }
                STATE.store(REGISTERED, Ordering::Release);
            }
        }

        #[inline]
        pub(crate) fn token() -> u64 {
            if STATE.load(Ordering::Acquire) != REGISTERED {
                register();
            }
            GENERATION.load(Ordering::Relaxed)
        }
    }

    #[cfg(all(
        unix,
        not(any(
            target_os = "linux",
            target_os = "android",
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly",
            target_os = "illumos",
            target_os = "solaris",
        ))
    ))]
    mod imp {
        #[inline]
        pub(crate) fn token() -> u64 {
            u64::from(std::process::id())
        }
    }

    #[cfg(not(unix))]
    mod imp {
        #[inline]
        pub(crate) fn token() -> u64 {
            0
        }
    }

    pub(crate) use imp::token;
}

/// The current fork token. Equal tokens mean "same process image".
#[inline]
pub(crate) fn fork_token() -> u64 {
    fork::token()
}

struct Pool {
    buf: [u8; POOL_LEN],
    pos: usize,
    fork_token: u64,
}

impl Pool {
    const fn empty() -> Self {
        Self {
            buf: [0; POOL_LEN],
            pos: POOL_LEN,
            fork_token: 0,
        }
    }

    #[inline]
    fn take<const N: usize>(&mut self) -> Result<[u8; N], RandomError> {
        let token = fork_token();
        if token != self.fork_token {
            // This buffer was copied from the parent by `fork()`. The
            // parent may hand out the same bytes, so drop them all.
            self.buf.fill(0);
            self.pos = POOL_LEN;
            self.fork_token = token;
        }
        if POOL_LEN - self.pos < N {
            refill(&mut self.buf)?;
            self.pos = 0;
        }
        let end = self.pos + N;
        let mut out = [0u8; N];
        out.copy_from_slice(&self.buf[self.pos..end]);
        self.buf[self.pos..end].fill(0);
        self.pos = end;
        Ok(out)
    }
}

#[cold]
fn refill(buf: &mut [u8]) -> Result<(), RandomError> {
    #[cfg(test)]
    if test_hooks::should_fail() {
        return Err(RandomError {
            source: std::io::Error::other("forced failure (test hook)"),
        });
    }
    mod_rand::tier3::fill_bytes(buf).map_err(|source| RandomError { source })
}

thread_local! {
    static POOL: RefCell<Pool> = const { RefCell::new(Pool::empty()) };
}

/// Panic with the error's message. Used by the panicking generators.
#[cold]
#[track_caller]
pub(crate) fn fail(e: RandomError) -> ! {
    panic!("{e}")
}

/// Draw one `u64` from the OS random source.
#[cfg(any(feature = "ulid", feature = "nanoid"))]
#[inline]
pub(crate) fn try_next_u64() -> Result<u64, RandomError> {
    POOL.with(|p| p.borrow_mut().take::<8>())
        .map(u64::from_le_bytes)
}

/// Draw 16 bytes from the OS random source.
#[cfg(feature = "uuid")]
#[inline]
pub(crate) fn try_next_bytes_16() -> Result<[u8; 16], RandomError> {
    POOL.with(|p| p.borrow_mut().take::<16>())
}

/// Test-only switch that makes the next refills fail on this thread.
#[cfg(test)]
pub(crate) mod test_hooks {
    use std::cell::Cell;

    thread_local! {
        static FAIL: Cell<bool> = const { Cell::new(false) };
    }

    pub(crate) fn should_fail() -> bool {
        FAIL.with(Cell::get)
    }

    /// Make refills on this thread fail until `set_fail(false)`, and
    /// empty this thread's pool so the next draw has to refill.
    pub(crate) fn set_fail(on: bool) {
        FAIL.with(|f| f.set(on));
        super::POOL.with(|p| p.borrow_mut().pos = super::POOL_LEN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take16() -> [u8; 16] {
        POOL.with(|p| p.borrow_mut().take::<16>()).unwrap()
    }

    #[test]
    fn draws_differ() {
        let a = take16();
        let b = take16();
        assert_ne!(a, b);
        assert_ne!(a, [0; 16]);
    }

    #[test]
    fn draws_cross_refill_boundaries() {
        // 1000 draws of 16 and 8 bytes cross many refills; none may be
        // all zero (a missed refill would hand out wiped bytes).
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1000 {
            let a = take16();
            assert!(seen.insert(a));
            let b = POOL.with(|p| p.borrow_mut().take::<8>()).unwrap();
            assert_ne!(b, [0; 8]);
        }
    }

    #[test]
    fn consumed_bytes_are_wiped() {
        POOL.with(|p| {
            let mut p = p.borrow_mut();
            let _ = p.take::<16>().unwrap();
            let start = p.pos - 16;
            assert!(p.buf[start..p.pos].iter().all(|&b| b == 0));
        });
    }

    #[test]
    fn changed_fork_token_discards_unread_bytes() {
        POOL.with(|p| {
            let mut p = p.borrow_mut();
            let _ = p.take::<16>().unwrap();
            // The bytes the next draw would hand out.
            let pending: [u8; 16] = p.buf[p.pos..p.pos + 16].try_into().unwrap();
            // Pretend this pool was copied from another process.
            p.fork_token = p.fork_token.wrapping_add(1);
            let next = p.take::<16>().unwrap();
            assert_ne!(next, pending, "a pool copied by fork must not be reused");
            assert_eq!(p.fork_token, fork_token());
            assert_eq!(p.pos, 16, "the pool must have been refilled from the start");
        });
    }

    #[test]
    fn failed_refill_is_an_error_and_recovers() {
        test_hooks::set_fail(true);
        let err = POOL.with(|p| p.borrow_mut().take::<16>()).unwrap_err();
        assert!(err.to_string().contains("OS random source failed"));
        assert!(std::error::Error::source(&err).is_some());
        // Still failing on the next call, without handing out bytes.
        assert!(POOL.with(|p| p.borrow_mut().take::<16>()).is_err());
        test_hooks::set_fail(false);
        assert_ne!(take16(), [0; 16]);
    }
}
