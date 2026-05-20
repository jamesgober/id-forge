//! NanoID alphabet validation.
//!
//! Demonstrates:
//!   * `nanoid::try_custom` — strict entry point that rejects empty
//!     and duplicate-byte alphabets
//!   * `nanoid::validate_alphabet` — call once at startup to vet a
//!     configuration value before the hot path
//!   * The difference between the permissive `custom` and the
//!     strict `try_custom`
//!
//! Run with: `cargo run --release --example nanoid_validate`

use id_forge::nanoid::{self, AlphabetError};

fn main() {
    println!("== try_custom: happy path ==");
    let id = nanoid::try_custom(16, b"0123456789abcdef").unwrap();
    println!("  {id}");

    println!("\n== try_custom: empty alphabet ==");
    match nanoid::try_custom(8, b"") {
        Ok(s) => println!("  unexpected ok: {s}"),
        Err(e) => println!("  Err: {e}"),
    }

    println!("\n== try_custom: duplicate byte ==");
    match nanoid::try_custom(8, b"abcda") {
        Ok(s) => println!("  unexpected ok: {s}"),
        Err(AlphabetError::Duplicate(b)) => {
            println!("  Err: duplicate byte 0x{b:02x} ({:?})", b as char);
        }
        Err(e) => println!("  Err: {e}"),
    }

    println!("\n== validate_alphabet for startup-time config check ==");
    let alphabets: &[(&str, &[u8])] = &[
        (
            "url-safe-64",
            b"_-0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
        ),
        ("hex", b"0123456789abcdef"),
        ("empty", b""),
        ("dup", b"aab"),
    ];
    for (label, alphabet) in alphabets {
        match nanoid::validate_alphabet(alphabet) {
            Ok(()) => println!("  {label:<14} OK ({} chars)", alphabet.len()),
            Err(e) => println!("  {label:<14} REJECTED: {e}"),
        }
    }

    println!("\n== Permissive `custom` tolerates duplicates ==");
    // Duplicates skew the output toward repeated chars. `custom`
    // accepts this by design; `try_custom` does not.
    let skewed = nanoid::custom(20, b"AAAAAA");
    println!("  custom(20, b\"AAAAAA\") = {skewed}");

    println!("\n== Cache a validated alphabet once at startup ==");
    let alphabet = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    nanoid::validate_alphabet(alphabet).expect("startup: config alphabet must be valid");
    // ... hot path now uses `custom` since the alphabet has already
    // been vetted; no per-call validation cost.
    for _ in 0..3 {
        println!("  {}", nanoid::custom(12, alphabet));
    }
}
