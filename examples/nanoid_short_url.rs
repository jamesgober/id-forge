//! NanoID for URL-shortener-style short IDs.
//!
//! Demonstrates:
//!   * `nanoid::with_length` for shorter-than-default codes
//:   * `nanoid::custom` with alphabets tailored to readability
//!     (no ambiguous characters like 0/O, 1/l/I)
//!   * Collision math at common lengths and rates
//!
//! Run with: `cargo run --release --example nanoid_short_url`

use id_forge::nanoid;
use std::collections::HashSet;

/// Crockford-flavoured alphabet — 32 chars, no 0/O/I/L/U
/// confusables, safe to read aloud.
const READABLE_32: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";

/// URL-safe lowercase 36-char alphabet.
const URL_36: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

fn main() {
    println!("== Default 21-char URL-safe IDs ==");
    for _ in 0..5 {
        println!("  {}", nanoid::generate());
    }

    println!("\n== Short codes (8 chars, URL-safe default) ==");
    for _ in 0..5 {
        println!("  {}", nanoid::with_length(8));
    }

    println!("\n== Readable alphabet (no 0/O/I/L/U), 10 chars ==");
    println!("  alphabet has {} chars", READABLE_32.len());
    for _ in 0..5 {
        println!("  {}", nanoid::custom(10, READABLE_32));
    }

    println!("\n== URL-safe lowercase, 12 chars ==");
    for _ in 0..5 {
        println!("  {}", nanoid::custom(12, URL_36));
    }

    println!("\n== Collision sweep at length 8 on URL_36 ==");
    let n = 100_000;
    let mut seen = HashSet::with_capacity(n);
    let mut collisions = 0usize;
    for _ in 0..n {
        if !seen.insert(nanoid::custom(8, URL_36)) {
            collisions += 1;
        }
    }
    println!("  drew {n} IDs of length 8");
    println!("  collisions = {collisions}");
    println!("  (URL_36^8 = {} ids/space)", (URL_36.len() as u64).pow(8));
}
