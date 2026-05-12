# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/jamesgober/id-forge/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jamesgober/id-forge/releases/tag/v0.1.0
