//! Snowflake deep dive — distributed worker IDs and decomposition.
//!
//! Demonstrates:
//!   * `Snowflake::new` for a single-host worker
//!   * `Snowflake::with_epoch` for a custom epoch (e.g. the Twitter
//!     2010 epoch)
//!   * `Snowflake::parts` to decode any ID back into
//!     `(timestamp_offset, worker, sequence)`
//!   * Multi-threaded contention producing globally unique IDs
//!
//! Run with: `cargo run --release --example snowflake_distributed`

use id_forge::snowflake::{Snowflake, DEFAULT_EPOCH_MS};
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;

fn main() {
    println!("== Default epoch (2026-01-01) ==");
    let gen = Snowflake::new(7);
    let id = gen.next_id();
    let (ts_offset, worker, seq) = Snowflake::parts(id);
    let wall_ms = ts_offset + gen.epoch_ms();
    println!("id           = {id}");
    println!("worker       = {worker}");
    println!("seq          = {seq}");
    println!("ts_offset_ms = {ts_offset}");
    println!("wall_ms      = {wall_ms}");
    println!("default epoch = {DEFAULT_EPOCH_MS}");

    println!("\n== Twitter's original 2010 epoch ==");
    let twitter_epoch_ms = 1_288_834_974_657;
    let tw = Snowflake::with_epoch(9, twitter_epoch_ms);
    let tw_id = tw.next_id();
    let (tw_ts, tw_worker, _) = Snowflake::parts(tw_id);
    println!("id           = {tw_id}");
    println!("worker       = {tw_worker}");
    println!("ts_offset_ms = {tw_ts}");
    println!("wall_ms      = {}", tw_ts + tw.epoch_ms());

    println!("\n== Multi-thread contention (8 threads x 2000 IDs) ==");
    let gen = Arc::new(Snowflake::new(3));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let g = Arc::clone(&gen);
            thread::spawn(move || (0..2000).map(|_| g.next_id()).collect::<Vec<_>>())
        })
        .collect();

    let mut all = HashSet::new();
    for h in handles {
        for id in h.join().unwrap() {
            all.insert(id);
        }
    }
    println!("expected     = {}", 8 * 2000);
    println!("unique seen  = {}", all.len());
    println!("no duplicates= {}", all.len() == 8 * 2000);

    println!("\n== Worker ID clamping ==");
    let clamped = Snowflake::new(0xFFFF);
    println!(
        "constructor input 0xFFFF -> worker_id() = {}",
        clamped.worker_id()
    );
    println!("(10-bit max = {})", 0x3FF);
}
