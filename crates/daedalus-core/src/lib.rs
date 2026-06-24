//! daedalus-core — the deterministic kernel of the Daedalus agent foundry.
//!
//! This crate is the Rust port of the Python `runner/` package, migrated one
//! parity-verified module at a time (leaf modules with clean file-format I/O
//! contracts first, the `pi`/OpenRouter boundary last). Each module ships
//! behind a parity oracle that runs the original Python and this port over
//! identical fixtures and asserts the verdicts agree.
//!
//! Migration status, module DAG, and strategy: `docs/rust-migration.md`.

pub mod cerberus;
pub mod cerberus_lab;
pub mod compare; // backlog 041: offline two-run delta (`daedalus compare`)
pub mod doctor;
pub mod export;
pub mod forecast; // backlog 041: offline cost/scale forecast (`run --estimate`)
pub mod judge;
pub mod launch;
pub mod lineage;
pub mod mutate;
pub mod port_harbor;
pub mod prompt_packet;
pub mod pycompat;
pub mod pyrandom;
pub mod report;
pub mod report_html; // backlog 044: self-contained static HTML report (lab.css)
pub mod run;
pub mod score;
pub mod search_loop; // ports runner/loop.py (`loop` is a Rust keyword)
pub mod seed;
pub mod stats; // backlog 039: confidence-bounded reward deltas (cluster-robust SE)
pub mod swarm;
pub mod taxonomy;
pub mod trace;
pub mod view; // backlog 049: live terminal dashboard over a run in flight
pub mod workbench;
