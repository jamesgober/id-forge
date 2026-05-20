# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-05-20

### Stable API

The `0.9.x` cycle delivered every algorithm; `1.0.0` is the
**stability commitment**. Every public item is now committed
under strict SemVer per [`docs/STABILITY.md`](docs/STABILITY.md).
Consumers can pin `id-forge = "1"` and expect minor / patch
updates within `1.x` to remain backwards-compatible.

### Added

- `docs/STABILITY.md` enumerating the frozen public surface, the
  behavioural contracts, and the items that are explicitly NOT
  part of the SemVer promise (internal PRNG choice, error `Display`
  text, transitive deps).
- `docs/API.md` — full API reference mirroring the `metrics-lib`
  format: example-pointers index, table of contents, installation
  + feature-flag matrix, an error-handling and panic-guarantees
  table, a section per public type (constructors, accessors,
  parsing, behavioural contract, multiple per-method examples),
  five real-world walkthroughs (time-ordered DB primary keys, URL
  shortener, distributed worker IDs, clock-skew recovery,
  migration from the `uuid` / `ulid` crates), and a performance
  summary.
- Rustdoc examples on `Uuid::as_bytes`, `Uuid::version`,
  `Ulid::as_bytes`, `Snowflake::worker_id`, and `Snowflake::epoch_ms`.
  Every public function and type now has an example per the
  pre-`1.0` directive.
- Seven new per-scheme examples in `examples/`:
  `uuid_v4`, `uuid_v7`, `ulid_monotonic`,
  `snowflake_distributed`, `snowflake_clock_skew`,
  `nanoid_short_url`, `nanoid_validate`. Each declares its
  `required-features` in `Cargo.toml` so `--no-default-features`
  CI runs skip them cleanly.

### Changed

- No source change to algorithm behaviour. Bytes a `Uuid::v4()`
  emits are identical to `0.9.3`.

### Frozen surface — summary

* **uuid**: `Uuid` + `nil/max/v4/v7/from_bytes/as_bytes/version/parse_str`
  + `ParseError`. Implements `Debug, Clone, Copy, PartialEq, Eq,
  Hash, PartialOrd, Ord, Default, Display, FromStr`.
* **ulid**: `Ulid` + `nil/max/new/from_bytes/as_bytes/timestamp_ms/parse_str`
  + `ParseError`. Implements `Debug, Clone, Copy, PartialEq, Eq,
  Hash, PartialOrd, Ord, Default, Display, FromStr`.
* **snowflake**: `Snowflake` + `new/with_epoch/worker_id/epoch_ms/try_next_id/next_id/parts`
  + `ClockSkew` + constants `DEFAULT_EPOCH_MS`, `SEQUENCE_BITS`,
  `WORKER_BITS`, `TIMESTAMP_BITS`.
* **nanoid**: `generate/with_length/custom/try_custom/validate_alphabet`
  + `AlphabetError` + constants `DEFAULT_ALPHABET`, `DEFAULT_LENGTH`.

### MSRV

Rust **1.75**, frozen at `1.0.0`. Within `1.x`, MSRV bumps are
advertised but not treated as breaking.

### Verification

Run on Windows x86_64, rustc 1.95; the full CI matrix
(ubuntu-latest, macos-latest, windows-latest) passes the same
gate:

```
cargo fmt --all -- --check
cargo clippy --all-targets --all-features    -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo build  --verbose
cargo build  --all-features --verbose
cargo build  --no-default-features --verbose
cargo test   --verbose
cargo test   --all-features --verbose
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
cargo +1.75 build --all-features --verbose
```

Test coverage at this release:

* 73 unit tests across `rng`, `uuid`, `ulid`, `snowflake`, `nanoid`.
* 16 smoke tests covering every public constructor.
* 35 doctests on the public API examples (every public function /
  type now has at least one).

## [0.9.3] - 2026-05-20

### Added

- `nanoid::try_custom(length, alphabet) -> Result<String, AlphabetError>`
  — strict counterpart to `custom`. Validates the alphabet (non-empty,
  no duplicate bytes) before generating.
- `nanoid::validate_alphabet(&[u8]) -> Result<(), AlphabetError>` so
  callers can vet a configuration alphabet once at startup.
- `nanoid::AlphabetError` enum (`Empty`, `Duplicate(u8)`) implementing
  `Display` and `std::error::Error`.
- `examples/bench.rs` — dep-free single-thread throughput harness for
  all four schemes. Run with `cargo run --release --example bench`.
  Uses `std::time::Instant`; no Criterion, no external dependency.

### Changed

- `nanoid::custom`, `with_length`, and `generate` now draw from the
  shared `crate::rng` xoshiro256\*\* generator instead of the 0.1.0
  LCG placeholder.
- Character selection switched from `byte % alphabet.len()` to a
  bitstream with a smallest-power-of-two mask and rejection sampling.
  Result: every character in any non-power-of-two alphabet has the
  same probability of being chosen. The 0.1.0 placeholder was
  measurably biased on a 17-character alphabet; the new
  implementation passes a ±12 % uniformity band on 170 000 samples.
- A length of `0` now short-circuits to the empty string instead of
  entering the generation loop.
- A single-character alphabet is treated specially (every output
  character is that single byte) instead of falling into a no-op
  loop.

### Tests

- 10 000-ID uniqueness sweep on the default 21-character alphabet.
- Bias check: 170 000 characters drawn over a 17-char alphabet, each
  position's frequency stays within ±12 % of the uniform expectation.
- `try_custom` rejects empty alphabets and the first duplicate byte
  encountered.
- `validate_alphabet` exposed as a public helper with its own tests.
- `mask_bits` lookup table verified for 2, 8, 64, 65, 256.
- Length round-trip across alphabet sizes {2, 7, 16, 33, 64, 65, 128, 200}.

### Notes

`nanoid::custom` remains permissive: empty alphabet returns `""`,
duplicate bytes are tolerated (and bias the distribution toward the
repeated bytes — by design, since this is the unchecked entry point).
Callers who want validation use `try_custom`.

## [0.9.2] - 2026-05-20

### Added

- `Snowflake::try_next_id() -> Result<u64, ClockSkew>` — explicit
  clock-skew handling. Returns `Err` when the wall clock has moved
  backward since the most recent ID; the existing `next_id()` keeps
  its `-> u64` signature and panics on the same condition (panic
  message names the millisecond offsets involved).
- `Snowflake::parts(id) -> (timestamp_offset_ms, worker_id, sequence)`
  — `const fn` decomposition for any Snowflake-layout ID, no matter
  which generator produced it. The timestamp is the offset from the
  generator's epoch; callers add `Snowflake::epoch_ms()` to get the
  wall-clock millisecond.
- `Snowflake::worker_id()` and `Snowflake::epoch_ms()` accessors.
- `ClockSkew` error type carrying `last_ms` and `now_ms` (offsets
  from the generator's epoch), implementing `Display` and
  `std::error::Error`.
- Public constants `SEQUENCE_BITS`, `WORKER_BITS`, `TIMESTAMP_BITS`
  so callers can spell the layout instead of hard-coding `12`, `10`,
  `41` in their own decoders.

### Changed

- `Snowflake::next_id` is no longer a per-call counter+time
  approximation. It runs a lock-free CAS loop over a packed
  `(last_ms, next_seq)` atomic word: monotonic within a worker, no
  duplicate IDs even under heavy multi-thread contention, sequence
  resets to 0 at each new millisecond.
- Sequence exhaustion (4096 IDs in a single millisecond) now blocks
  the calling thread in a microsleep loop until the wall clock
  advances, instead of issuing duplicate sequence numbers. Wait is
  bounded by one millisecond minus elapsed time in the current ms.
- `Snowflake::new` and `Snowflake::with_epoch` are now `const fn`.

### Tests

- 10 000-ID monotonic burst on a single generator.
- 50 000-ID uniqueness sweep.
- 8 threads × 2 000 IDs = 16 000 IDs under contention, all unique.
- Forced clock-skew scenario via direct state manipulation:
  `try_next_id` returns `Err(ClockSkew)`, `next_id` panics.
- Sequence-exhaustion scenario: pre-load the state to mark a
  millisecond exhausted and confirm the next call blocks until the
  next millisecond and starts the new ms at sequence 0.
- `parts` round-trip and per-field extraction against a hand-built
  ID.

## [0.9.1] - 2026-05-20

### Added

- `Ulid::new` real implementation per the [ULID spec]: 48-bit
  big-endian millisecond timestamp prefix + 80 bits of randomness
  from the shared xoshiro256\*\* generator.
- **Monotonic factory.** Two ULIDs minted in the same millisecond by
  the same process are strictly byte-wise ordered: the second one is
  the first one's random suffix plus one. Cross-millisecond, fresh
  randomness is drawn. Implements the "monotonic" guarantee from the
  spec.
- `Ulid::nil()` (all-zeros) and `Ulid::max()` (`7ZZZZZZZZZZZZZZZZZZZZZZZZZ`,
  the largest valid 128-bit encoded value).
- `Ulid::from_bytes(&[u8; 16])` to wrap an existing big-endian
  representation.
- `Ulid::parse_str(&str)` and `impl FromStr for Ulid`. Case-insensitive,
  honours Crockford substitutions (`I`/`L` -> `1`, `O` -> `0`); `U`
  is reserved and rejected. Returns `ParseError` with the failing
  byte position or an `Overflow` variant when the leading character
  exceeds 7 (which would set the 129th bit).
- `Ulid::timestamp_ms()` accessor for the 48-bit millisecond prefix.
- `Default` for `Ulid` (returns `nil`).
- `PartialOrd` / `Ord` on `Ulid`.

### Changed

- ULID random source: 0.1.0 counter+time placeholder -> shared
  xoshiro256\*\* PRNG. The 80-bit suffix is now genuinely random across
  milliseconds; within a millisecond it advances by +1 for strict
  monotonicity.
- Crockford base32 `Display` rewritten as a `u128`-shift loop — the
  previous implementation was already correct but built per-nibble on
  every iteration; the new one is one branch-free pass.
- Internal: extracted `Xoshiro256SS` into a private `crate::rng`
  module now that both `uuid` and `ulid` consume it. UUID generation
  is byte-identical to 0.9.0.

### Tests

- ULID spec round-trip on `01ARZ3NDEKTSV4RRFFQ69G5FAV`.
- Crockford substitutions: `I`, `L`, `O` decode to `1`/`1`/`0`; `U`
  is rejected.
- Leading-character overflow (`8…`, `Z…`) is reported as `Overflow`.
- 10 000-ULID uniqueness sweep.
- 1000-burst monotonicity check (strictly increasing within a process).

[ULID spec]: https://github.com/ulid/spec

## [0.9.0] - 2026-05-20

### Added

- `Uuid::v4` real implementation per RFC 9562 §5.4: 122 random bits with
  version `0100` and RFC 4122 variant bits.
- `Uuid::v7` real implementation per RFC 9562 §5.7: 48-bit big-endian
  millisecond timestamp prefix, 74 random bits, version `0111`, RFC 4122
  variant bits, byte-wise time-ordered across milliseconds.
- `Uuid::nil()` (all-zeros, §5.9) and `Uuid::max()` (all-ones, §5.10).
- `Uuid::from_bytes(&[u8; 16])` to wrap an existing big-endian
  representation without touching version/variant bits.
- `Uuid::parse_str(&str)` and `impl FromStr for Uuid` for the canonical
  36-character hyphenated form, case-insensitive. Errors expose the
  failing byte position via `ParseError`.
- `Uuid::version()` accessor for the high nibble of byte 6.
- `Default` for `Uuid` (returns `nil`).
- `PartialOrd` / `Ord` on `Uuid` so v7 IDs sort byte-wise by timestamp.
- Inline xoshiro256\*\* PRNG seeded from process ID, wall-clock
  nanoseconds, and a per-process counter via SplitMix64. Thread-local,
  no syscall after the first draw of a thread.

### Changed

- UUID random source switched from the 0.1.0 counter+time placeholder to
  the xoshiro256\*\* generator. Two consecutive `Uuid::v4` calls now
  differ in all 122 random bits, not just the trailing counter.

### Tests

- RFC 9562 Appendix A.2 (v4) and A.6 (v7) example strings parse and
  round-trip with the correct version/variant bits.
- 10 000-UUID uniqueness sweep on `v4`.
- v7 IDs generated 2ms apart compare strictly byte-wise.
- `parse_str` rejects wrong length, missing hyphens, and non-hex digits
  with the failing position reported.

### Notes

The randomness is fast non-cryptographic. For session tokens or API
keys, compose with a CSPRNG for the random portion — `id-forge` itself
intentionally has no `getrandom` dependency.

### CI

- `Cargo.toml` declares `required-features` on the `basic` example and
  the `smoke` integration test (all four scheme features). This makes
  `cargo clippy --all-targets --no-default-features` pass: Cargo
  simply skips the targets whose feature set is not active, instead of
  trying to compile them against missing modules.

## [0.1.0] - 2026-05-11

### Added

- Initial crate skeleton.
- `uuid` module: `Uuid::v4` and `Uuid::v7` (placeholder implementations).
- `ulid` module: `Ulid::new` with Crockford-base32 Display impl
  (placeholder timestamp/randomness mix).
- `snowflake` module: `Snowflake::new`, `with_epoch`, `next_id`
  (placeholder per-ms sequence logic).
- `nanoid` module: `generate`, `with_length`, `custom` (placeholder
  randomness).
- Feature flags: `std` (default), `uuid`, `ulid`, `snowflake`, `nanoid`.
- Smoke tests for each scheme.

### Note

This is the name-claim release. Real implementations follow RFC 9562
for UUIDs, the ULID spec, and the Twitter Snowflake design. Production
randomness lands in `0.9.x`.

[Unreleased]: https://github.com/jamesgober/id-forge/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/jamesgober/id-forge/releases/tag/v1.0.0
[0.9.3]: https://github.com/jamesgober/id-forge/releases/tag/v0.9.3
[0.9.2]: https://github.com/jamesgober/id-forge/releases/tag/v0.9.2
[0.9.1]: https://github.com/jamesgober/id-forge/releases/tag/v0.9.1
[0.9.0]: https://github.com/jamesgober/id-forge/releases/tag/v0.9.0
[0.1.0]: https://github.com/jamesgober/id-forge/releases/tag/v0.1.0
