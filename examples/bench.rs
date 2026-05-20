//! Throughput benchmark for every scheme in `id-forge`.
//!
//! Single-threaded. Uses `std::time::Instant` — no Criterion, no
//! external dependency. Intended for quick smoke-grade measurements
//! and regression checks against the [`REPS`] performance targets, not
//! statistically rigorous reporting.
//!
//! Run with:
//!
//! ```text
//! cargo run --release --example bench
//! ```
//!
//! [`REPS`]: ../../REPS.md

use id_forge::{nanoid, snowflake::Snowflake, ulid::Ulid, uuid::Uuid};
use std::time::Instant;

fn bench<R>(name: &str, iters: usize, mut f: impl FnMut() -> R) {
    for _ in 0..(iters / 10).max(1) {
        let _ = f();
    }
    let start = Instant::now();
    for _ in 0..iters {
        let _ = f();
    }
    let elapsed = start.elapsed();
    let per_op_ns = elapsed.as_nanos() as f64 / iters as f64;
    let throughput = 1_000_000_000.0 / per_op_ns;
    println!("{name:<32} {iters:>9} iters  {per_op_ns:>8.1} ns/op   {throughput:>10.0} ops/s");
}

fn main() {
    println!("id-forge throughput (single thread, release build)");
    println!("---------------------------------------------------");

    let iters = 1_000_000;
    bench("Uuid::v4", iters, Uuid::v4);
    bench("Uuid::v7", iters, Uuid::v7);
    bench("Ulid::new", iters, Ulid::new);

    let sf = Snowflake::new(1);
    bench("Snowflake::next_id", iters, || sf.next_id());

    bench("nanoid::generate", iters / 5, nanoid::generate);
    bench("nanoid::with_length(8)", iters / 5, || {
        nanoid::with_length(8)
    });
    bench("nanoid::custom(16, hex)", iters / 5, || {
        nanoid::custom(16, b"0123456789abcdef")
    });
    bench("nanoid::custom(21, 17-char)", iters / 5, || {
        nanoid::custom(21, b"ABCDEFGHIJKLMNOPQ")
    });
}
