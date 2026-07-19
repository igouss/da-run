//! `da-state` — the run-state authority, as a library.
//!
//! The binary in `main.rs` is a thin shell over [`exec::execute`]. The split
//! exists so the selftest ladder is a `cargo test` first and a CLI ritual
//! second — the same walk, one source, no drift between what CI proves and
//! what `da-state selftest` claims.

pub mod cli;
pub mod exec;
pub mod pretty;
pub mod selftest;
