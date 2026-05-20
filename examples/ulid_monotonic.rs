//! ULID monotonic factory deep dive.
//!
//! Demonstrates:
//!   * `Ulid::new` per the ULID spec
//!   * Strict monotonicity inside a single millisecond
//!   * `timestamp_ms` accessor and Crockford base32 round-trip
//!
//! Run with: `cargo run --release --example ulid_monotonic`

use id_forge::ulid::Ulid;
use std::collections::HashSet;

fn main() {
    println!("== Single-millisecond burst ==");

    // 1000 ULIDs back-to-back. Most will share the same ms prefix;
    // the monotonic factory guarantees b > a for every consecutive pair.
    let mut ids: Vec<Ulid> = (0..1000).map(|_| Ulid::new()).collect();

    let same_ms = ids
        .windows(2)
        .filter(|w| w[0].timestamp_ms() == w[1].timestamp_ms())
        .count();
    println!("count        = {}", ids.len());
    println!("same-ms pairs = {same_ms} of {}", ids.len() - 1);

    let strictly_increasing = ids.windows(2).all(|w| w[1] > w[0]);
    println!("monotonic    = {strictly_increasing}");

    let unique: HashSet<&Ulid> = ids.iter().collect();
    println!("unique       = {} of {}", unique.len(), ids.len());

    println!("\n== First and last of the burst ==");
    let first = ids.first().unwrap();
    let last = ids.last().unwrap();
    println!("first        = {first}");
    println!("last         = {last}");
    println!("ts_first     = {} ms", first.timestamp_ms());
    println!("ts_last      = {} ms", last.timestamp_ms());

    println!("\n== Crockford base32 round-trip ==");
    let sample = Ulid::new();
    let s = sample.to_string();
    let parsed = Ulid::parse_str(&s).unwrap();
    println!("sample       = {sample}");
    println!("parsed       = {parsed}");
    println!("equal        = {}", sample == parsed);

    println!("\n== Substitutions accepted by parse_str ==");
    let with_subs = Ulid::parse_str("0IIIIIIIIIIIIIIIIIIIIIIIII").unwrap();
    let canonical = Ulid::parse_str("01111111111111111111111111").unwrap();
    println!("\"0III...\" == \"0111...\" = {}", with_subs == canonical);

    println!("\n== Sorting by Display is sorting by time ==");
    ids.sort_by_key(|u| u.to_string());
    let same = ids.windows(2).all(|w| w[1] >= w[0]);
    println!("string sort  = byte sort = {same}");
}
