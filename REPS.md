# id-forge — Project Specification (REPS)

> Rust Engineering Project Specification.
> Normative language follows RFC 2119.

## 1. Purpose

`id-forge` MUST provide implementations of every commonly-used unique
ID scheme in a single zero-dependency library: UUID v4, UUID v7,
ULID, Snowflake, and NanoID.

## 2. Schemes

### UUID (uuid module)

- `Uuid::v4()` MUST produce RFC 9562 §5.4-compliant random UUIDs.
- `Uuid::v7()` MUST produce RFC 9562 §5.7-compliant time-ordered
  UUIDs with millisecond timestamp prefix and random suffix.
- `Display` impl MUST produce the canonical 36-character hyphenated
  form (e.g. `f47ac10b-58cc-4372-a567-0e02b2c3d479`).

### ULID (ulid module)

- `Ulid::new()` MUST produce a ULID per the spec:
  https://github.com/ulid/spec
- 48-bit millisecond timestamp prefix, 80-bit randomness suffix.
- `Display` impl MUST produce the 26-character Crockford-base32 form.

### Snowflake (snowflake module)

- `Snowflake::new(worker_id)` MUST produce a generator using the
  default 2026-01-01 epoch.
- `Snowflake::with_epoch(worker_id, epoch_ms)` MUST allow custom epochs.
- Bit layout: 41 bits timestamp + 10 bits worker ID + 12 bits sequence.
- `next_id()` MUST produce monotonically increasing IDs within a
  worker (sequence rolls over per millisecond).
- Worker IDs MUST be clamped to the 10-bit range.

### NanoID (nanoid module)

- `nanoid::generate()` MUST produce a 21-character ID using the
  default URL-safe alphabet.
- `nanoid::with_length(n)` MUST produce an `n`-character ID.
- `nanoid::custom(n, alphabet)` MUST accept any non-empty alphabet.

## 3. Determinism

All schemes that include a random component MUST produce unpredictable
output. Two consecutive calls MUST NOT produce the same value.

## 4. Dependencies

This crate MUST NOT have runtime dependencies outside `std`. No
external random crates, no UUID/ULID/Snowflake/NanoID crates.

## 5. Performance targets

- UUID v4 generation: <100ns per ID
- UUID v7 generation: <200ns per ID (includes time syscall)
- ULID generation: <200ns per ID
- Snowflake generation: <100ns per ID (no syscall after first call)
- NanoID generation: <500ns per 21-character ID

## 6. Cryptographic quality

For workloads requiring cryptographically unpredictable IDs (session
tokens, API keys), users SHOULD compose with `mod-rand::tier3` for
the random portions. `id-forge` itself uses fast non-crypto random
for performance reasons; this MUST be documented prominently.

## 7. Stability

Through `0.9.x` the public API MAY shift. The `1.0` release pins the
API and the wire format of each scheme (UUIDs and ULIDs are governed
by external specs and cannot change).

## 8. Out of scope

- ID parsing (string -> Uuid/Ulid/etc.) — deferred to `0.9.x` based
  on user demand.
- Database-specific ID generation (PostgreSQL UUID extensions, etc.).
- Encrypted/signed IDs (HMAC-based, etc.).
- Hierarchical IDs (Twitter Snowflake variants with extra fields).
