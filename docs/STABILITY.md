# id-forge — Stability Promise

> Frozen as of `v1.0.0`. Anything listed here is part of the
> committed public surface; removing it, renaming it, or changing
> its signature requires a `2.0` release. Additive changes (new
> variants, new methods, new types, new modules, new feature flags)
> are normal minor / patch releases.

`id-forge` follows **strict SemVer** within the `1.x` series:

* MAJOR (`2.0`) — breaking changes to any item below.
* MINOR (`1.y`) — additive changes only.
* PATCH (`1.x.y`) — bug fixes, internal optimisations, doc edits.

The wire format of each scheme — UUID bytes, ULID Crockford
base32 string, the Snowflake 41/10/12 bit layout, NanoID character
selection — is fixed by the upstream specs and cannot change in any
release.

## Frozen surface

### Crate-level

* Feature flags (current set is the committed set):
  `default` (= `std + uuid + ulid + snowflake + nanoid`),
  `std`, `uuid`, `ulid`, `snowflake`, `nanoid`.
* MSRV: **Rust 1.75**, frozen at `1.0.0`. MSRV bumps within `1.x`
  are advertised in the CHANGELOG and are **not** treated as
  breaking.
* `no_std` compatibility when the `std` feature is off (note: the
  current `uuid`, `ulid`, `snowflake`, and `nanoid` modules all
  require `std`; this is by design and matches the feature flags).

### `id_forge::uuid` (feature: `uuid`)

* `struct Uuid([u8; 16])` — public-by-value type, opaque body.
* `Uuid::nil() -> Self` (`const fn`)
* `Uuid::max() -> Self` (`const fn`)
* `Uuid::v4() -> Self`
* `Uuid::v7() -> Self`
* `Uuid::from_bytes(&[u8; 16]) -> Self` (`const fn`)
* `Uuid::as_bytes(&self) -> &[u8; 16]` (`const fn`)
* `Uuid::version(&self) -> u8` (`const fn`)
* `Uuid::parse_str(&str) -> Result<Self, ParseError>`
* Trait impls: `Debug, Clone, Copy, PartialEq, Eq, Hash,
  PartialOrd, Ord, Default, Display, FromStr`
* `enum ParseError` with variants `InvalidLength(usize)`,
  `InvalidGroup(usize)`, `InvalidChar(usize)`. Implements
  `Debug, Clone, Copy, PartialEq, Eq, Display, std::error::Error`.

### `id_forge::ulid` (feature: `ulid`)

* `struct Ulid([u8; 16])` — public-by-value type, opaque body.
* `Ulid::nil() -> Self` (`const fn`)
* `Ulid::max() -> Self` (`const fn`)
* `Ulid::new() -> Self`
* `Ulid::from_bytes(&[u8; 16]) -> Self` (`const fn`)
* `Ulid::as_bytes(&self) -> &[u8; 16]` (`const fn`)
* `Ulid::timestamp_ms(&self) -> u64` (`const fn`)
* `Ulid::parse_str(&str) -> Result<Self, ParseError>`
* Trait impls: `Debug, Clone, Copy, PartialEq, Eq, Hash,
  PartialOrd, Ord, Default, Display, FromStr`
* `enum ParseError` with variants `InvalidLength(usize)`,
  `InvalidChar(usize)`, `Overflow`. Implements
  `Debug, Clone, Copy, PartialEq, Eq, Display, std::error::Error`.

### `id_forge::snowflake` (feature: `snowflake`)

* `struct Snowflake { worker_id, epoch_ms, state }`
* `Snowflake::new(u16) -> Self` (`const fn`)
* `Snowflake::with_epoch(u16, u64) -> Self` (`const fn`)
* `Snowflake::worker_id(&self) -> u16` (`const fn`)
* `Snowflake::epoch_ms(&self) -> u64` (`const fn`)
* `Snowflake::try_next_id(&self) -> Result<u64, ClockSkew>`
* `Snowflake::next_id(&self) -> u64` — panics on clock skew.
* `Snowflake::parts(u64) -> (u64, u16, u16)` (`const fn`)
* `struct ClockSkew { pub last_ms: u64, pub now_ms: u64 }` with
  `Debug, Clone, Copy, PartialEq, Eq, Display, std::error::Error`.
* Constants: `DEFAULT_EPOCH_MS`, `SEQUENCE_BITS`, `WORKER_BITS`,
  `TIMESTAMP_BITS`.

### `id_forge::nanoid` (feature: `nanoid`)

* `nanoid::generate() -> String`
* `nanoid::with_length(usize) -> String`
* `nanoid::custom(usize, &[u8]) -> String`
* `nanoid::try_custom(usize, &[u8]) -> Result<String, AlphabetError>`
* `nanoid::validate_alphabet(&[u8]) -> Result<(), AlphabetError>`
* `enum AlphabetError` with variants `Empty`, `Duplicate(u8)`.
  Implements `Debug, Clone, Copy, PartialEq, Eq, Display,
  std::error::Error`.
* Constants: `DEFAULT_ALPHABET`, `DEFAULT_LENGTH`.

## Behavioural contracts

These are part of the freeze. Future releases will not regress
them:

* **`Uuid::v4` produces RFC 9562 §5.4 random UUIDs.** Version
  nibble = `0100`, variant bits = `10`. The 122 random bits come
  from a non-cryptographic generator; callers needing CSPRNG output
  must compose their own.
* **`Uuid::v7` produces RFC 9562 §5.7 time-ordered UUIDs.** 48-bit
  big-endian millisecond prefix; version nibble = `0111`; variant
  bits = `10`. Two v7 UUIDs minted in different milliseconds
  compare byte-wise in timestamp order.
* **`Ulid::new` produces ULID-spec-compliant 26-character Crockford
  base32 strings.** 48-bit ms prefix + 80 random bits. The
  **monotonic factory** guarantee: two ULIDs minted in the same
  millisecond by the same process are strictly byte-wise ordered;
  the second one is the first one's random suffix `+ 1`.
* **`Snowflake::next_id` is strictly monotonic per worker.** No
  duplicates under multi-thread contention. Sequence exhaustion
  blocks the calling thread in a microsleep loop until the wall
  clock advances. Clock regression causes a `Result::Err(ClockSkew)`
  from `try_next_id` and a panic from `next_id`.
* **`nanoid::custom`, `with_length`, `generate` are bias-free.**
  Power-of-two-mask rejection sampling. Every byte in an alphabet
  has identical probability of being chosen.
* **Parse / Display round-trip.** For every `Uuid` and `Ulid`,
  `parse_str(&x.to_string()) == Ok(x)`.

## What is NOT promised

* The exact internal PRNG (currently xoshiro256\*\*) — a future
  patch release may switch to a faster generator if it improves
  throughput without weakening the bias / uniqueness guarantees.
* Error `Display` strings — the format may change between minor
  releases for clarity.
* Internal performance characteristics beyond the targets in
  `REPS.md §5`. The `examples/bench.rs` numbers are
  informational, not contractual.
* The transitive dependency tree — `id-forge` itself has no runtime
  dependencies outside `std`, and that is contractual. Anything
  Cargo adds for tooling (`dev-dependencies`, build deps) can
  change.
* `#[doc(hidden)]` items and any `#[cfg(test)]` modules. These are
  not part of the public surface even if visible.
* The Snowflake CAS state layout. Callers must use `parts()` to
  decode IDs.

## Compatibility note

`id-forge` ships under `Apache-2.0 OR MIT` dual license. Anything
in `1.x` that breaks the promises above is a bug; please file an
issue against [id-forge](https://github.com/jamesgober/id-forge/issues).
