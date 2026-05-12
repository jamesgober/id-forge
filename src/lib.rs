//! # id-forge
//!
//! Typed, high-performance unique ID generation for Rust. Every common
//! ID scheme in one zero-dependency library:
//!
//! - **[`uuid`]**: UUID v4 (random) and v7 (time-ordered)
//! - **[`ulid`]**: Universally Unique Lexicographically Sortable ID
//! - **[`snowflake`]**: Twitter Snowflake-style 64-bit IDs (epoch + worker + sequence)
//! - **[`nanoid`]**: URL-safe random strings of any length
//!
//! ## Quick example
//!
//! ```
//! use id_forge::uuid::Uuid;
//!
//! let id = Uuid::v4();
//! println!("{id}");
//! ```
//!
//! ## Why this library exists
//!
//! Today's options are fragmented: `uuid` for UUIDs, `ulid` for ULIDs,
//! `snowflake-rs` for snowflakes, `nanoid` for NanoIDs. Each has its
//! own quirks, MSRV, and dependencies. `id-forge` is one zero-dep
//! crate at MSRV 1.75 covering every scheme.
//!
//! ## Status
//!
//! `v0.1.0` is the name-claim release with placeholder implementations.
//! The real algorithms land in `0.9.x`.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

#[cfg(feature = "uuid")]
pub mod uuid;

#[cfg(feature = "ulid")]
pub mod ulid;

#[cfg(feature = "snowflake")]
pub mod snowflake;

#[cfg(feature = "nanoid")]
pub mod nanoid;
