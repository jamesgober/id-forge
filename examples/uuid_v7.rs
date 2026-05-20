//! UUID v7 deep dive — time-ordered random UUIDs.
//!
//! Demonstrates:
//!   * `Uuid::v7` (RFC 9562 §5.7 — 48-bit ms prefix + 74 random bits)
//!   * Byte-wise sort order matches time order
//!   * Suitability as a database primary key
//!
//! Run with: `cargo run --release --example uuid_v7`

use id_forge::uuid::Uuid;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    println!("== UUID v7 time-ordered IDs ==");

    let mut ids = Vec::new();
    for i in 0..5 {
        let id = Uuid::v7();
        println!("#{i} {id}");
        ids.push(id);
        sleep(Duration::from_millis(2));
    }

    println!("\n== Byte-wise sort matches creation order ==");
    let mut sorted = ids.clone();
    sorted.sort();
    println!("same order   = {}", sorted == ids);

    println!("\n== Version field ==");
    println!("first.version() = {}", ids[0].version());

    println!("\n== As a database primary key ==");
    // v7 IDs cluster recent inserts in B-tree leaves — better cache
    // locality than v4 for time-windowed queries.
    let rows: Vec<(Uuid, &str)> = vec![
        (Uuid::v7(), "alice signed up"),
        (Uuid::v7(), "bob signed up"),
        (Uuid::v7(), "carol signed up"),
    ];
    println!("Insert order (recent rows live near the tip of the index):");
    for (id, evt) in &rows {
        println!("  {id}  {evt}");
    }
}
