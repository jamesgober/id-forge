//! UUID v4 deep dive — random IDs and round-trip parsing.
//!
//! Demonstrates:
//!   * `Uuid::v4` (random, RFC 9562 §5.4)
//!   * `Uuid::nil` and `Uuid::max`
//!   * `parse_str` round-trip with each kind of `ParseError`
//!   * `as_bytes` storage round-trip
//!
//! Run with: `cargo run --release --example uuid_v4`

use id_forge::uuid::{ParseError, Uuid};

fn main() {
    println!("== UUID v4 ==");

    let a = Uuid::v4();
    let b = Uuid::v4();
    println!("a            = {a}");
    println!("b            = {b}");
    println!("a == b       = {}", a == b);
    println!("a.version()  = {}", a.version());

    println!("\n== Nil and Max ==");
    println!("nil          = {}", Uuid::nil());
    println!("max          = {}", Uuid::max());

    println!("\n== parse_str round-trip ==");
    let canonical = "f47ac10b-58cc-4372-a567-0e02b2c3d479";
    let parsed = Uuid::parse_str(canonical).expect("canonical UUID parses");
    println!("parse(\"{canonical}\") = {parsed}");
    println!("round-trip ok = {}", parsed.to_string() == canonical);

    println!("\n== Case-insensitive parse ==");
    let upper = "F47AC10B-58CC-4372-A567-0E02B2C3D479";
    let lower_round = Uuid::parse_str(upper).unwrap();
    println!("upper input  -> {lower_round}  (Display is always lowercase)");

    println!("\n== ParseError variants ==");
    show_err("too short", Uuid::parse_str("abc"));
    show_err(
        "missing hyphen",
        Uuid::parse_str("f47ac10b_58cc-4372-a567-0e02b2c3d479"),
    );
    show_err(
        "non-hex digit",
        Uuid::parse_str("g47ac10b-58cc-4372-a567-0e02b2c3d479"),
    );

    println!("\n== Byte storage round-trip ==");
    let id = Uuid::v4();
    let bytes: [u8; 16] = *id.as_bytes();
    let restored = Uuid::from_bytes(&bytes);
    println!("id           = {id}");
    println!("restored     = {restored}");
    println!("equal        = {}", id == restored);
}

fn show_err(label: &str, result: Result<Uuid, ParseError>) {
    match result {
        Ok(id) => println!("{label:<18} OK ({id})"),
        Err(e) => println!("{label:<18} Err({e})"),
    }
}
