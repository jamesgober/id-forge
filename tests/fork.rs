//! A child created with `fork()` must not repeat the parent's random IDs.
//!
//! The parent primes its per-thread random buffer and ULID state, then
//! forks. Parent and child each mint the same mix of IDs, and the child
//! sends its IDs back over a socket pair. No ID may appear on both sides.

#![cfg(unix)]

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use id_forge::nanoid;
use id_forge::ulid::Ulid;
use id_forge::uuid::Uuid;

extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

/// The IDs each side mints after the fork, as one string per ID.
fn mint() -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..4 {
        out.push(Uuid::v4().to_string());
    }
    for _ in 0..4 {
        out.push(nanoid::generate());
    }
    out.push(Ulid::new().to_string());
    out.push(Uuid::v7().to_string());
    out.push(Uuid::try_v4().unwrap().to_string());
    out.push(Ulid::try_new().unwrap().to_string());
    out
}

#[test]
fn forked_child_does_not_repeat_parent_ids() {
    // Prime this thread's buffer and ULID state so the fork copies a
    // partly used buffer and a live monotonic counter.
    let _ = Uuid::v4();
    let _ = Ulid::new();
    let _ = nanoid::generate();

    let (mut parent_end, mut child_end) = UnixStream::pair().expect("socket pair");

    // SAFETY: the child only mints IDs, writes them to its socket and
    // calls `_exit`. It never returns into the test harness.
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        drop(parent_end);
        let ids = mint().join("\n");
        let ok = child_end.write_all(ids.as_bytes()).is_ok();
        drop(child_end);
        // SAFETY: ends the child without running the parent's
        // destructors or the test harness.
        unsafe { _exit(if ok { 0 } else { 1 }) };
    }
    drop(child_end);

    let parent_ids = mint();
    let mut buf = String::new();
    parent_end.read_to_string(&mut buf).expect("read child IDs");
    let mut status = 0;
    // SAFETY: `pid` is our own child; `status` is a valid pointer.
    let waited = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(waited, pid);
    assert_eq!(status, 0, "child exited with status {status}");

    let child_ids: Vec<&str> = buf.lines().collect();
    assert_eq!(child_ids.len(), parent_ids.len(), "child sent every ID");
    let shared: Vec<&String> = parent_ids
        .iter()
        .filter(|id| child_ids.contains(&id.as_str()))
        .collect();
    assert!(
        shared.is_empty(),
        "{} of {} IDs repeated in parent and child: {shared:?}",
        shared.len(),
        parent_ids.len()
    );
}
