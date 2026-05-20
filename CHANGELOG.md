# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/jamesgober/id-forge/compare/v0.9.1...HEAD
[0.9.1]: https://github.com/jamesgober/id-forge/releases/tag/v0.9.1
[0.9.0]: https://github.com/jamesgober/id-forge/releases/tag/v0.9.0
[0.1.0]: https://github.com/jamesgober/id-forge/releases/tag/v0.1.0
