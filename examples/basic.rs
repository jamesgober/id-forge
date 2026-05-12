//! Minimal example: generate one of each ID type.
//!
//! Run with: `cargo run --example basic`

use id_forge::{nanoid, snowflake::Snowflake, ulid::Ulid, uuid::Uuid};

fn main() {
    println!("UUID v4:    {}", Uuid::v4());
    println!("UUID v7:    {}", Uuid::v7());
    println!("ULID:       {}", Ulid::new());

    let gen = Snowflake::new(1);
    println!("Snowflake:  {}", gen.next_id());

    println!("NanoID 21:  {}", nanoid::generate());
    println!("NanoID 8:   {}", nanoid::with_length(8));
}
