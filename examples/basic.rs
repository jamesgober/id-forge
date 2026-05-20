//! Minimal example: generate one of each ID type.
//!
//! Run with: `cargo run --example basic`

use id_forge::{nanoid, snowflake::Snowflake, ulid::Ulid, uuid::Uuid};

fn main() {
    let v4 = Uuid::v4();
    println!("UUID v4:    {v4} (version={})", v4.version());
    println!("UUID v7:    {}", Uuid::v7());
    println!("UUID nil:   {}", Uuid::nil());

    let a = Ulid::new();
    let b = Ulid::new();
    println!("ULID a:     {a}");
    println!("ULID b:     {b} (monotonic: {})", b > a);

    let gen = Snowflake::new(1);
    let sf = gen.next_id();
    let (ts_offset, worker, seq) = Snowflake::parts(sf);
    println!(
        "Snowflake:  {sf}  (ts+epoch={}, worker={worker}, seq={seq})",
        ts_offset + gen.epoch_ms()
    );

    println!("NanoID 21:  {}", nanoid::generate());
    println!("NanoID 8:   {}", nanoid::with_length(8));

    assert_eq!(v4, Uuid::parse_str(&v4.to_string()).unwrap());
    assert_eq!(a, Ulid::parse_str(&a.to_string()).unwrap());
}
