<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <strong>id-forge</strong>
    <br>
    <sup><sub>UNIQUE ID GENERATION FOR RUST</sub></sup>
</h1>

<p align="center">
    <a href="https://crates.io/crates/id-forge"><img alt="crates.io" src="https://img.shields.io/crates/v/id-forge.svg"></a>
    <a href="https://crates.io/crates/id-forge"><img alt="downloads" src="https://img.shields.io/crates/d/id-forge.svg"></a>
    <a href="https://docs.rs/id-forge"><img alt="docs.rs" src="https://docs.rs/id-forge/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md" title="MSRV"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.75%2B-blue"></a>
    <a href="https://github.com/jamesgober/id-forge/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/jamesgober/id-forge/actions/workflows/ci.yml/badge.svg"></a>
</p>

<p align="center">
    UUID v4/v7, ULID, Snowflake, NanoID — every common scheme in one zero-dependency library.<br>
    Monotonic, distributed-safe, sortable variants.
</p>

---

## What it does

Every project needs unique identifiers. Today's options force you to
mix and match crates: `uuid` for UUIDs, `ulid` for ULIDs,
`snowflake-rs` for Snowflakes, `nanoid` for NanoIDs. Each has its
own conventions, MSRV, and dependency tree.

`id-forge` puts every common ID scheme in one crate with zero
runtime dependencies and MSRV 1.75.

## Quick start

```rust
use id_forge::{uuid::Uuid, ulid::Ulid, snowflake::Snowflake, nanoid};

let id = Uuid::v4();                     // "f47ac10b-58cc-4372-..."
let id = Uuid::v7();                     // time-ordered UUID
let id = Ulid::new();                    // "01H6X3VPK..."
let gen = Snowflake::new(1);             // worker ID = 1
let id: u64 = gen.next_id();             // 64-bit snowflake
let id = nanoid::generate();             // "VNqJgL1..."
let id = nanoid::with_length(8);         // shorter NanoID
```

## Schemes supported

| Scheme | Size | Sortable | Distributed-safe | Use case |
|--------|------|----------|------------------|----------|
| UUID v4 | 128 bits | No | Yes | Random identifiers, no time component needed |
| UUID v7 | 128 bits | Yes | Yes | Time-ordered random ID (preferred over v4 for DB primary keys) |
| ULID | 128 bits | Yes | Yes | Sortable, URL-friendly identifiers |
| Snowflake | 64 bits | Yes | Yes (with worker ID) | Fits in `u64`, time-ordered, suited for high-throughput |
| NanoID | configurable | No | Yes | URL-safe short IDs, configurable alphabet |

## Feature flags

```toml
[dependencies]
id-forge = "0.1"                                         # all schemes (default)
id-forge = { version = "0.1", default-features = false } # nothing (compile errors if you use anything)
id-forge = { version = "0.1", default-features = false, features = ["uuid"] }  # just UUIDs
```

## Minimum supported Rust version

`1.75` — pinned in `Cargo.toml` and verified by CI.


<br>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.


<!-- FOOT COPYRIGHT
################################################# -->
<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>JAMES GOBER.</strong></sup>
</div>