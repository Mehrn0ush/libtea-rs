//! Compiles the Rust snippets of the workspace `README.md` as doctests, so
//! they cannot drift from the API. Nothing else lives here.
//!
//! This crate is unpublished (`publish = false`), so including a file from
//! outside its directory does not affect packaging. No published crate
//! depends on it.

#![forbid(unsafe_code)]

/// The workspace `README.md`.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
pub struct RootReadmeDoctests;
