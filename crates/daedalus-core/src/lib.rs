//! daedalus-core — the deterministic kernel of the Daedalus agent foundry.
//!
//! This crate is the Rust port of the Python `runner/` package, migrated one
//! parity-verified module at a time (leaf modules with clean file-format I/O
//! contracts first, the `pi`/OpenRouter boundary last). Each module ships
//! behind a parity oracle that runs the original Python and this port over
//! identical fixtures and asserts the verdicts agree.
//!
//! Migration status, module DAG, and strategy: `docs/rust-migration.md`.

pub mod score;
