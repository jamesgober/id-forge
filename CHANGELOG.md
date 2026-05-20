# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/jamesgober/id-forge/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/jamesgober/id-forge/releases/tag/v0.9.0
[0.1.0]: https://github.com/jamesgober/id-forge/releases/tag/v0.1.0
