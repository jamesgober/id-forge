<h1 align="center">
        <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br><b>id-forge</b><br>
    <sub><sup>API REFERENCE</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;│&nbsp;</span>
        <span>API</span>
        <span>&nbsp;│&nbsp;</span>
        <a href="./STABILITY.md" title="Stability Promise"><b>STABILITY</b></a>
        <span>&nbsp;│&nbsp;</span>
        <a href="../CHANGELOG.md" title="Changelog"><b>CHANGELOG</b></a>
    </sup>
</div>
<br>

<h4 id="example-pointers">Example Pointers</h4>

- All schemes in one file: `examples/basic.rs` — one line per scheme.
- Single-thread throughput: `examples/bench.rs` — `std::time::Instant`, no Criterion.
- UUID v4 deep dive: `examples/uuid_v4.rs` — random IDs, parse errors, byte round-trip.
- UUID v7 deep dive: `examples/uuid_v7.rs` — time-ordered IDs as DB primary keys.
- ULID monotonic factory: `examples/ulid_monotonic.rs` — 1000 IDs in one ms, strictly ordered.
- Snowflake distributed: `examples/snowflake_distributed.rs` — multi-worker, multi-thread, `parts()` decode.
- Snowflake clock skew: `examples/snowflake_clock_skew.rs` — `try_next_id` recovery.
- NanoID short URLs: `examples/nanoid_short_url.rs` — readable alphabets, collision sweep.
- NanoID validation: `examples/nanoid_validate.rs` — `try_custom`, `validate_alphabet`.

Run any of them with `cargo run --release --example <name>`.

## Table of Contents

- **[Installation](#installation)**
- **[Quick Start](#quick-start)**
- **[Feature Flags](#feature-flags)**
- **[Error handling and panic guarantees](#error-handling-and-panic-guarantees)**
- **[Public APIs](#public-apis)**
  - [`uuid::Uuid`](#uuiduuid)
  - [`uuid::ParseError`](#uuidparseerror)
  - [`ulid::Ulid`](#ulidulid)
  - [`ulid::ParseError`](#ulidparseerror)
  - [`snowflake::Snowflake`](#snowflakesnowflake)
  - [`snowflake::ClockSkew`](#snowflakeclockskew)
  - [`snowflake` constants](#snowflake-constants)
  - [`nanoid` free functions](#nanoid-free-functions)
  - [`nanoid::AlphabetError`](#nanoidalphabeterror)
  - [`nanoid` constants](#nanoid-constants)
- **[Real-World Examples](#real-world-examples)**
  - [Time-ordered database primary keys (v7 / ULID)](#real-world-time-ordered-pk)
  - [URL shortener with NanoID](#real-world-url-shortener)
  - [Distributed Snowflake worker IDs across hosts](#real-world-distributed-workers)
  - [Surfacing clock skew as a service-level error](#real-world-clock-skew)
  - [Switching from `uuid` / `ulid` crates](#real-world-migration)
- **[Performance](#performance)**
- **[Stability](#stability)**

---

## Installation

```toml
[dependencies]
id-forge = "1"
```

Per-scheme subsets:

```toml
# Just UUIDs
id-forge = { version = "1", default-features = false, features = ["uuid"] }

# UUID + ULID
id-forge = { version = "1", default-features = false, features = ["uuid", "ulid"] }

# Snowflake only
id-forge = { version = "1", default-features = false, features = ["snowflake"] }

# NanoID only
id-forge = { version = "1", default-features = false, features = ["nanoid"] }
```

MSRV: Rust **1.75**. Zero runtime dependencies outside `std`.

## Quick Start

```rust
use id_forge::{uuid::Uuid, ulid::Ulid, snowflake::Snowflake, nanoid};

fn main() {
    // Random and time-ordered UUIDs (RFC 9562)
    let v4: Uuid = Uuid::v4();
    let v7: Uuid = Uuid::v7();

    // ULID — sortable Crockford base32, monotonic per ms
    let ulid: Ulid = Ulid::new();

    // 64-bit Snowflake — 41-bit ms + 10-bit worker + 12-bit seq
    let gen = Snowflake::new(1);
    let snowflake: u64 = gen.next_id();

    // URL-safe short IDs
    let short: String = nanoid::generate();        // 21 chars
    let custom: String = nanoid::with_length(8);   // 8 chars

    println!("{v4} {v7} {ulid} {snowflake} {short} {custom}");
}
```

## Feature Flags

| Flag        | Default? | Pulls in                              |
|-------------|:--------:|---------------------------------------|
| `default`   | yes      | `std + uuid + ulid + snowflake + nanoid` |
| `std`       | yes      | Activates `SystemTime`, `String`, `Vec` paths. |
| `uuid`      | yes      | `id_forge::uuid` module.              |
| `ulid`      | yes      | `id_forge::ulid` module.              |
| `snowflake` | yes      | `id_forge::snowflake` module.         |
| `nanoid`    | yes      | `id_forge::nanoid` module.            |

The feature set is part of the public API per `docs/STABILITY.md`. New flags are minor releases; existing flags cannot be renamed or removed without a major bump.

## Error handling and panic guarantees

| Method                              | Failure mode                       | Recovery                              |
|-------------------------------------|------------------------------------|---------------------------------------|
| `Uuid::parse_str`                   | `Err(uuid::ParseError)`            | Inspect variant; reject input.        |
| `Ulid::parse_str`                   | `Err(ulid::ParseError)`            | Inspect variant; reject input.        |
| `Snowflake::try_next_id`            | `Err(ClockSkew)` on backward clock | Sleep `drift+1 ms`, retry.            |
| `Snowflake::next_id`                | **Panic** on backward clock        | Catch with `try_next_id` instead.     |
| `nanoid::try_custom`                | `Err(AlphabetError)`               | Pre-validate alphabet at startup.     |
| `nanoid::custom`                    | Silent (empty/dup-tolerant)        | Use `try_custom` if rejection wanted. |

`id-forge` **never** issues an ID that could collide with one previously issued by the same generator. When the wall clock cannot guarantee that, `try_next_id` returns `Err` and `next_id` panics — neither falls back to a possibly-duplicate value.

---

## Public APIs

### `uuid::Uuid`

Source: `src/uuid.rs`. Feature: `uuid`.

A 128-bit UUID stored as `[u8; 16]` in big-endian network order, identical to the wire layout in RFC 9562. `Display` produces the canonical 36-character hyphenated form (lowercase). `FromStr` accepts the same form case-insensitively.

**Trait impls:** `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Display, FromStr`.

#### Constructors

- `Uuid::nil() -> Self` (`const fn`) — all-zeros UUID (RFC 9562 §5.9).
- `Uuid::max() -> Self` (`const fn`) — all-ones UUID (RFC 9562 §5.10).
- `Uuid::v4() -> Self` — RFC 9562 §5.4 random UUID. 122 random bits + version `0100` + RFC 4122 variant.
- `Uuid::v7() -> Self` — RFC 9562 §5.7 time-ordered UUID. 48-bit big-endian millisecond prefix + 74 random bits + version `0111` + RFC 4122 variant.
- `Uuid::from_bytes(&[u8; 16]) -> Self` (`const fn`) — wrap an existing big-endian byte representation without modifying version / variant bits.

#### Accessors

- `Uuid::as_bytes(&self) -> &[u8; 16]` (`const fn`) — borrow the raw 16-byte representation.
- `Uuid::version(&self) -> u8` (`const fn`) — high nibble of byte 6. `4` for v4, `7` for v7, `0` for `nil`, `15` for `max`.

#### Parsing

- `Uuid::parse_str(&str) -> Result<Self, ParseError>` — accepts the canonical 36-character hyphenated form, case-insensitive.
- `<Uuid as FromStr>::from_str` — same.

#### Example: generate, render, and parse

```rust
use id_forge::uuid::Uuid;

let id = Uuid::v4();
let s = id.to_string();                              // "f47ac10b-58cc-4372-..."
assert_eq!(s.len(), 36);
assert_eq!(Uuid::parse_str(&s).unwrap(), id);
```

#### Example: time-ordered IDs sort byte-wise

```rust
use id_forge::uuid::Uuid;
use std::thread::sleep;
use std::time::Duration;

let a = Uuid::v7();
sleep(Duration::from_millis(2));
let b = Uuid::v7();
assert!(b > a);              // byte-wise sort matches creation order
```

#### Example: round-trip through `[u8; 16]`

```rust
use id_forge::uuid::Uuid;

let id = Uuid::v4();
let bytes: [u8; 16] = *id.as_bytes();
let restored = Uuid::from_bytes(&bytes);
assert_eq!(id, restored);
```

#### Example: case-insensitive parsing

```rust
use id_forge::uuid::Uuid;

let upper = "F47AC10B-58CC-4372-A567-0E02B2C3D479";
let id = Uuid::parse_str(upper).unwrap();
assert_eq!(id.to_string(), "f47ac10b-58cc-4372-a567-0e02b2c3d479");
```

### `uuid::ParseError`

```rust
pub enum ParseError {
    InvalidLength(usize),
    InvalidGroup(usize),
    InvalidChar(usize),
}
```

`InvalidLength(n)` — input was not exactly 36 characters; `n` is the actual length.

`InvalidGroup(p)` — a hyphen was missing at byte position `p` (expected 8, 13, 18, or 23).

`InvalidChar(p)` — a non-hex digit was found at byte position `p`.

Implements `Debug, Clone, Copy, PartialEq, Eq, Display, std::error::Error`.

#### Example: pattern-match on each variant

```rust
use id_forge::uuid::{ParseError, Uuid};

match Uuid::parse_str("abc") {
    Err(ParseError::InvalidLength(n))  => println!("length was {n}"),
    Err(ParseError::InvalidGroup(p))   => println!("missing hyphen at {p}"),
    Err(ParseError::InvalidChar(p))    => println!("bad char at {p}"),
    Ok(_)                              => unreachable!(),
}
```

---

### `ulid::Ulid`

Source: `src/ulid.rs`. Feature: `ulid`.

A 128-bit ULID: 48-bit big-endian millisecond timestamp + 80-bit randomness, displayed as 26 Crockford base32 characters. The `Display` order is identical to byte-wise sort order and to creation-time order.

**Trait impls:** `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Display, FromStr`.

#### Constructors

- `Ulid::nil() -> Self` (`const fn`) — all-zeros.
- `Ulid::max() -> Self` (`const fn`) — all-ones (`7ZZZZZZZZZZZZZZZZZZZZZZZZZ`).
- `Ulid::new() -> Self` — current millisecond + 80 fresh random bits, with the per-process monotonic factory guarantee (see below).
- `Ulid::from_bytes(&[u8; 16]) -> Self` (`const fn`) — wrap raw bytes.

#### Accessors

- `Ulid::as_bytes(&self) -> &[u8; 16]` (`const fn`)
- `Ulid::timestamp_ms(&self) -> u64` (`const fn`) — the 48-bit ms prefix.

#### Parsing

- `Ulid::parse_str(&str) -> Result<Self, ParseError>` — case-insensitive; Crockford substitutions (`I`/`L` → `1`, `O` → `0`) are honoured; `U` is rejected.
- `<Ulid as FromStr>::from_str` — same.

#### Monotonic factory

The factory is thread-local and consists of `(last_ms, last_rand)`. When `new()` observes the same millisecond as the previous call on the same thread, it returns `last_rand + 1` instead of fresh randomness. This guarantees `b > a` for every consecutive pair in a process, even when thousands of IDs are minted within a single millisecond.

#### Example: in-millisecond burst, all strictly ordered

```rust
use id_forge::ulid::Ulid;

let ids: Vec<Ulid> = (0..100).map(|_| Ulid::new()).collect();
assert!(ids.windows(2).all(|w| w[1] > w[0]));
// They almost certainly share the same ms prefix; the +1
// suffix makes them strictly ordered anyway.
```

#### Example: render and parse round-trip

```rust
use id_forge::ulid::Ulid;

let id = Ulid::parse_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap();
assert_eq!(id.to_string(), "01ARZ3NDEKTSV4RRFFQ69G5FAV");
```

#### Example: Crockford substitutions

```rust
use id_forge::ulid::Ulid;

let with_subs = Ulid::parse_str("0IIIIIIIIIIIIIIIIIIIIIIIII").unwrap();
let canonical = Ulid::parse_str("01111111111111111111111111").unwrap();
assert_eq!(with_subs, canonical);              // I -> 1
```

#### Example: extract the timestamp prefix

```rust
use id_forge::ulid::Ulid;

let id = Ulid::new();
let ms = id.timestamp_ms();
println!("minted at {} ms since UNIX epoch", ms);
```

### `ulid::ParseError`

```rust
pub enum ParseError {
    InvalidLength(usize),
    InvalidChar(usize),
    Overflow,
}
```

`InvalidLength(n)` — input was not exactly 26 characters.

`InvalidChar(p)` — character at position `p` is not a valid Crockford digit (or it's `U`, which is reserved).

`Overflow` — the leading character encodes a value above `7`, which would set bits beyond the 128-bit range.

Implements `Debug, Clone, Copy, PartialEq, Eq, Display, std::error::Error`.

---

### `snowflake::Snowflake`

Source: `src/snowflake.rs`. Feature: `snowflake`.

64-bit IDs in the Twitter Snowflake layout. Lock-free CAS state machine packs `(last_ms, next_seq)` into a single `AtomicU64`.

```text
 63                   22         12         0
 +---------------------+----------+----------+
 | 41 bits ms offset   | 10 bits  | 12 bits  |
 | (since epoch_ms)    | worker   | sequence |
 +---------------------+----------+----------+
```

**Trait impls:** `Debug`. The type is `!Clone` by design (each generator owns its own monotonic state; cloning would create a parallel issuer that could collide). Wrap in `Arc<Snowflake>` to share across threads.

#### Constructors

- `Snowflake::new(worker_id: u16) -> Self` (`const fn`) — uses the default 2026-01-01T00:00:00Z epoch (`DEFAULT_EPOCH_MS`).
- `Snowflake::with_epoch(worker_id: u16, epoch_ms: u64) -> Self` (`const fn`) — custom epoch.

Worker IDs above 1023 are silently clamped to 10 bits (`worker_id & 0x3FF`).

#### Accessors

- `Snowflake::worker_id(&self) -> u16` (`const fn`) — the (possibly clamped) worker ID.
- `Snowflake::epoch_ms(&self) -> u64` (`const fn`) — the epoch this generator subtracts from the wall clock.

#### Generation

- `Snowflake::try_next_id(&self) -> Result<u64, ClockSkew>` — explicit clock-skew handling. Blocks the calling thread in a 100 µs sleep loop when the millisecond's 4096-ID sequence is exhausted; that block ends as soon as the wall clock advances.
- `Snowflake::next_id(&self) -> u64` — convenience wrapper around `try_next_id`. **Panics** on `Err(ClockSkew)`.

#### Decoding

- `Snowflake::parts(id: u64) -> (u64, u16, u16)` (`const fn`) — decompose any Snowflake-layout ID into `(timestamp_offset_ms, worker_id, sequence)`. The first element is the offset from the originating generator's epoch; add `epoch_ms()` to get the wall-clock millisecond.

#### Behavioural contract (frozen in `1.0`)

1. **Monotonic per generator.** Two IDs in program order satisfy `id_b > id_a`. Holds across threads, ms boundaries, and sequence rollover.
2. **No duplicates per generator.** CAS retries serialise the issuance; sequence exhaustion blocks rather than recycling.
3. **No bogus IDs on clock skew.** Both `try_next_id` and `next_id` refuse to issue.

#### Example: simple usage

```rust
use id_forge::snowflake::Snowflake;

let gen = Snowflake::new(1);
let a = gen.next_id();
let b = gen.next_id();
assert!(b > a);
```

#### Example: explicit error handling

```rust
use id_forge::snowflake::{ClockSkew, Snowflake};

let gen = Snowflake::new(1);
match gen.try_next_id() {
    Ok(id) => println!("issued {id}"),
    Err(ClockSkew { last_ms, now_ms }) => {
        eprintln!("clock drifted {} ms backward; retry after a pause",
                  last_ms - now_ms);
    }
}
```

#### Example: decode any ID

```rust
use id_forge::snowflake::Snowflake;

let gen = Snowflake::new(7);
let id = gen.next_id();
let (ts_offset, worker, seq) = Snowflake::parts(id);
let wall_ms = ts_offset + gen.epoch_ms();
assert_eq!(worker, 7);
println!("wall-clock ms = {wall_ms}, sequence = {seq}");
```

#### Example: multi-thread issuance

```rust
use id_forge::snowflake::Snowflake;
use std::sync::Arc;
use std::thread;

let gen = Arc::new(Snowflake::new(3));
let handles: Vec<_> = (0..4).map(|_| {
    let g = Arc::clone(&gen);
    thread::spawn(move || (0..1000).map(|_| g.next_id()).collect::<Vec<_>>())
}).collect();

let mut all = Vec::new();
for h in handles { all.extend(h.join().unwrap()); }
all.sort();
all.dedup();
assert_eq!(all.len(), 4 * 1000);     // every ID is unique
```

### `snowflake::ClockSkew`

```rust
pub struct ClockSkew {
    pub last_ms: u64,
    pub now_ms: u64,
}
```

Both fields are offsets from the generator's epoch (not wall-clock ms). `now_ms < last_ms` is what produced the error.

Implements `Debug, Clone, Copy, PartialEq, Eq, Display, std::error::Error`.

### `snowflake` constants

- `DEFAULT_EPOCH_MS: u64 = 1_767_225_600_000` — 2026-01-01T00:00:00Z in ms since UNIX epoch.
- `SEQUENCE_BITS: u32 = 12`
- `WORKER_BITS: u32 = 10`
- `TIMESTAMP_BITS: u32 = 41`

These let downstream code spell the bit layout without hard-coding `12`, `10`, `41`.

```rust
use id_forge::snowflake::{SEQUENCE_BITS, TIMESTAMP_BITS, WORKER_BITS};
assert_eq!(SEQUENCE_BITS + WORKER_BITS + TIMESTAMP_BITS, 63);
```

---

### `nanoid` free functions

Source: `src/nanoid.rs`. Feature: `nanoid`.

NanoID is exposed as free functions on the `nanoid` module. There is no struct — the output is `String`.

#### Functions

- `nanoid::generate() -> String` — 21-character ID over the default URL-safe alphabet. Equivalent to `nanoid::with_length(DEFAULT_LENGTH)`.
- `nanoid::with_length(length: usize) -> String` — `length`-character ID over the default alphabet.
- `nanoid::custom(length: usize, alphabet: &[u8]) -> String` — `length` characters over `alphabet`. **Permissive**: empty alphabet returns `""`, duplicate bytes are tolerated (and skew the output distribution toward repeated bytes).
- `nanoid::try_custom(length: usize, alphabet: &[u8]) -> Result<String, AlphabetError>` — strict counterpart. Validates the alphabet first.
- `nanoid::validate_alphabet(alphabet: &[u8]) -> Result<(), AlphabetError>` — vet an alphabet without generating anything. Useful for startup-time configuration checks.

#### Bias-free selection

Character selection is power-of-two-mask rejection sampling, not `byte % n`. For any alphabet `A`, every byte in `A` has identical probability of being chosen. Acceptance rate is `|A| / 2^ceil(log2(|A|))`: 100% for a 64-character alphabet, 53% for a 17-character alphabet, 78% for a 200-character alphabet. The output distribution is uniform in all cases.

#### Example: default 21-character IDs

```rust
use id_forge::nanoid;

let id = nanoid::generate();
assert_eq!(id.len(), 21);
```

#### Example: shorter IDs

```rust
use id_forge::nanoid;

let token = nanoid::with_length(10);
assert_eq!(token.len(), 10);
```

#### Example: custom alphabet (hex)

```rust
use id_forge::nanoid;

let hex = nanoid::custom(16, b"0123456789abcdef");
assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
```

#### Example: readable alphabet (no 0/O/I/L/U)

```rust
use id_forge::nanoid;

const READABLE_32: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
let code = nanoid::custom(8, READABLE_32);
// "code" is safe to read aloud over the phone.
```

#### Example: validate at startup, generate in the hot path

```rust
use id_forge::nanoid;

const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn startup_check() {
    nanoid::validate_alphabet(ALPHABET)
        .expect("config: alphabet must be valid");
}

fn hot_path() -> String {
    // No per-call validation cost — alphabet was already vetted.
    nanoid::custom(12, ALPHABET)
}
```

#### Example: surface validation errors

```rust
use id_forge::nanoid::{self, AlphabetError};

match nanoid::try_custom(8, b"aab") {
    Ok(id) => println!("{id}"),
    Err(AlphabetError::Empty) => eprintln!("alphabet must be non-empty"),
    Err(AlphabetError::Duplicate(b)) => {
        eprintln!("alphabet contains duplicate byte 0x{b:02x}");
    }
}
```

### `nanoid::AlphabetError`

```rust
pub enum AlphabetError {
    Empty,
    Duplicate(u8),
}
```

`Empty` — alphabet was the empty slice.

`Duplicate(b)` — byte `b` appears more than once. The first duplicate found is reported.

Implements `Debug, Clone, Copy, PartialEq, Eq, Display, std::error::Error`.

### `nanoid` constants

- `DEFAULT_ALPHABET: &[u8]` — 64-character URL-safe set: `_-` + `0-9` + `a-z` + `A-Z`.
- `DEFAULT_LENGTH: usize = 21` — gives roughly one trillion years to a 1% collision probability at 1000 IDs/second.

```rust
use id_forge::nanoid::{DEFAULT_ALPHABET, DEFAULT_LENGTH};
assert_eq!(DEFAULT_ALPHABET.len(), 64);
assert_eq!(DEFAULT_LENGTH, 21);
```

---

## Real-World Examples

<h3 id="real-world-time-ordered-pk">Time-ordered database primary keys (v7 / ULID)</h3>

Both `Uuid::v7` and `Ulid::new` produce IDs whose byte-wise sort order matches creation time. Compared to `Uuid::v4`, this cuts B-tree fragmentation on heavily-inserted tables because recent rows live near the tip of the index.

```rust
use id_forge::ulid::Ulid;

struct User {
    id: Ulid,
    email: String,
}

fn create_user(email: String) -> User {
    User { id: Ulid::new(), email }
}

// In the DB layer, store as TEXT (the 26-char Crockford string) or
// BYTEA (`.as_bytes()`). Either form sorts in creation order.
let alice = create_user("alice@example.com".into());
let bob   = create_user("bob@example.com".into());
assert!(bob.id > alice.id);
```

Use **v7** when you need RFC 9562 compliance for cross-language clients. Use **ULID** when you want the shorter Crockford-base32 display and the monotonic-factory guarantee inside a single ms.

<h3 id="real-world-url-shortener">URL shortener with NanoID</h3>

A 7-character ID over a 36-symbol alphabet gives `36^7 ≈ 78` billion codes — comfortable margin for any URL shortener that's not Bitly-scale. Skip ambiguous characters (`0/O/I/L/U`) to make codes that survive being read aloud.

```rust
use id_forge::nanoid;
use std::collections::HashMap;

const READABLE: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ"; // 31 chars

struct Shortener {
    db: HashMap<String, String>,
}

impl Shortener {
    fn shorten(&mut self, url: &str) -> String {
        loop {
            let code = nanoid::custom(7, READABLE);
            if !self.db.contains_key(&code) {
                self.db.insert(code.clone(), url.into());
                return code;
            }
            // Loop on the astronomically rare collision.
        }
    }
}
```

<h3 id="real-world-distributed-workers">Distributed Snowflake worker IDs across hosts</h3>

The 10-bit worker field gives 1024 unique generators per cluster. Allocate distinct worker IDs at host startup — from configuration, from a registry like Consul, or from `K8s_POD_INDEX` for a StatefulSet:

```rust
use id_forge::snowflake::Snowflake;

fn build_snowflake_generator() -> Snowflake {
    let worker_id: u16 = std::env::var("WORKER_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("WORKER_ID env var must be 0..=1023");
    Snowflake::new(worker_id)
}
```

The generator is `Send + Sync`. Wrap it in `Arc` and share across the whole process. Two distinct hosts with distinct `WORKER_ID`s will never collide even if their clocks are millisecond-perfect.

<h3 id="real-world-clock-skew">Surfacing clock skew as a service-level error</h3>

Production services hit clock skew rarely but consistently — NTP corrections, VM migrations, container restarts. Use `try_next_id` to detect it, decide whether to retry, and surface it as a metric:

```rust
use id_forge::snowflake::{ClockSkew, Snowflake};
use std::thread::sleep;
use std::time::Duration;

fn issue_id(gen: &Snowflake) -> Result<u64, &'static str> {
    let mut attempts = 0;
    loop {
        match gen.try_next_id() {
            Ok(id) => return Ok(id),
            Err(ClockSkew { last_ms, now_ms }) => {
                let drift = last_ms - now_ms;
                if attempts >= 32 || drift > 5_000 {
                    return Err("clock too far back to recover");
                }
                sleep(Duration::from_millis(drift + 1));
                attempts += 1;
            }
        }
    }
}
```

For a drift of 1–2 ms (the common case), the retry pause matches the regression and `try_next_id` succeeds on the second attempt. Drifts beyond a few seconds usually indicate a misconfigured clock and are worth alerting on.

<h3 id="real-world-migration">Switching from <code>uuid</code> / <code>ulid</code> crates</h3>

`id-forge`'s 16-byte representation matches the `uuid` and `ulid` crates' wire format, so values round-trip both ways via `as_bytes()` / `from_bytes()`:

```rust
// Pretend `legacy::Uuid` is the upstream `uuid` crate type.
mod legacy {
    pub struct Uuid([u8; 16]);
    impl Uuid {
        pub fn as_bytes(&self) -> &[u8; 16] { &self.0 }
        pub fn from_bytes(b: [u8; 16]) -> Self { Self(b) }
    }
}

use id_forge::uuid::Uuid as IdfUuid;

fn from_legacy(u: &legacy::Uuid) -> IdfUuid {
    IdfUuid::from_bytes(u.as_bytes())
}

fn to_legacy(u: &IdfUuid) -> legacy::Uuid {
    legacy::Uuid::from_bytes(*u.as_bytes())
}
```

Same approach works for `ulid` — both crates store the 16-byte big-endian form.

---

## Performance

Numbers below are informational, not contractual. Single-thread, release build, Windows x86_64, rustc 1.95. Reproduce with `cargo run --release --example bench`.

| Scheme                     |  ns/op | REPS §5 target  |
|----------------------------|-------:|-----------------|
| `Uuid::v4`                 |    3.0 | <100 ns         |
| `Uuid::v7`                 |   18.3 | <200 ns         |
| `Ulid::new`                |   23.6 | <200 ns         |
| `Snowflake::next_id`       |  244.0 | <100 ns         |
| `nanoid::generate` (21 ch) |   37.7 | <500 ns / 21 ch |

`Snowflake::next_id` misses its target by ~2.4× because `SystemTime::now()` is called on every issuance to keep the monotonicity guarantee. This is the only target miss in the suite; a future patch may add an `Instant`-anchored fast path.

## Stability

Every item documented above is part of the **frozen public surface** as of `1.0.0`. See [`STABILITY.md`](./STABILITY.md) for the complete SemVer contract, MSRV pin, and the explicit list of items that are *not* covered (internal PRNG, error `Display` text, internal performance characteristics).

