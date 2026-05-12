<h1 align="center">
    <strong>id-forge</strong>
    <br>
    <sup><sub>UNIQUE ID GENERATION FOR RUST</sub></sup>
</h1>

<p align="center">
    <a href="https://crates.io/crates/id-forge"><img alt="crates.io" src="https://img.shields.io/crates/v/id-forge.svg"></a>
    <a href="https://crates.io/crates/id-forge"><img alt="downloads" src="https://img.shields.io/crates/d/id-forge.svg"></a>
    <a href="https://docs.rs/id-forge"><img alt="docs.rs" src="https://docs.rs/id-forge/badge.svg"></a>
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

## Status

`v0.1.0` is the name-claim release with placeholder implementations
of every scheme. The real algorithms (RFC 9562 UUIDs, full
Crockford-base32 ULIDs, RFC-compliant Snowflake sequence rollover,
NIST-quality NanoID randomness) land in `0.9.x`.

## Minimum supported Rust version

`1.75` — pinned in `Cargo.toml` and verified by CI.

## License

Apache-2.0. See [LICENSE](LICENSE).
