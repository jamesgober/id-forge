//! Minimal example: generate one of each ID type.
//!
//! Run with: `cargo run --example basic`

use id_forge::{nanoid, snowflake::Snowflake, ulid::Ulid, uuid::Uuid};

fn main() {
    let v4 = Uuid::v4();
    println!("UUID v4:    {v4} (version={})", v4.version());
    println!("UUID v7:    {}", Uuid::v7());
    println!("UUID nil:   {}", Uuid::nil());
    println!("ULID:       {}", Ulid::new());

    let gen = Snowflake::new(1);
    println!("Snowflake:  {}", gen.next_id());

    println!("NanoID 21:  {}", nanoid::generate());
    println!("NanoID 8:   {}", nanoid::with_length(8));

    let round_tripped = Uuid::parse_str(&v4.to_string()).unwrap();
    assert_eq!(v4, round_tripped);
}
