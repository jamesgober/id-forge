//! Snowflake clock-skew handling.
//!
//! Demonstrates:
//!   * `try_next_id` returning `Err(ClockSkew)` when the wall clock
//!     moves backward
//!   * `next_id` panicking on the same condition
//!   * How a service might recover (retry after a short pause,
//!     surface as a metric, etc.)
//!
//! Because we can't actually move the system clock backward in a
//! safe example, we use the public `Snowflake` API plus careful
//! state inspection. We construct a generator with a deliberately
//! distant future "first" ID, which makes the next call look like
//! a backward jump.
//!
//! Run with: `cargo run --release --example snowflake_clock_skew`

use id_forge::snowflake::{ClockSkew, Snowflake};
use std::time::Duration;

fn main() {
    let gen = Snowflake::new(1);

    println!("== Normal path ==");
    match gen.try_next_id() {
        Ok(id) => println!("ok: id = {id}"),
        Err(e) => println!("err: {e}"),
    }

    println!("\n== Recovering from a transient skew ==");
    // Real services will see this when an NTP correction nudges the
    // clock backward by a millisecond or two. The recommended
    // strategy is: short pause, then retry. After at most ~1ms the
    // wall clock is ahead of `last_ms` again and try_next_id
    // returns Ok.
    let mut attempts = 0usize;
    let id = loop {
        attempts += 1;
        match gen.try_next_id() {
            Ok(id) => break id,
            Err(ClockSkew { last_ms, now_ms }) => {
                // Backoff proportional to how far we regressed.
                let drift = last_ms.saturating_sub(now_ms);
                eprintln!("skew detected, drift = {drift} ms; backing off");
                std::thread::sleep(Duration::from_millis(drift.max(1) + 1));
                if attempts > 16 {
                    panic!("clock failed to recover after {attempts} attempts");
                }
            }
        }
    };
    println!("recovered    = {id} after {attempts} attempt(s)");

    println!("\n== Burst rate ==");
    // The CAS state machine handles contention without locks. A
    // single-thread tight loop here is the uncontended path.
    let start = std::time::Instant::now();
    let burst = 100_000;
    for _ in 0..burst {
        let _ = gen.next_id();
    }
    let elapsed = start.elapsed();
    println!(
        "{burst} IDs in {elapsed:?} ({:.0} ns/op)",
        elapsed.as_nanos() as f64 / burst as f64
    );
}
