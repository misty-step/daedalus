# Build the Rust validation kernel for schemas, receipts, and contracts

Priority: P1
Status: delivered
Estimate: L

## Goal

Resolve the reopened Phase 4 Rust-kernel decision by making schema validation,
receipt validation, and launch-contract tooling durable Rust-owned surfaces
instead of scattered compatibility checks.

## Why Now

`DESIGN.md` reopened this decision on 2026-06-12 after `pr-review-v2` and
`launch-contract-v0` proved that the schemas had survived two accepted task
families. `ROADMAP.md` still lists "Rust kernel for stable schemas, receipt
validation, contract tooling" as a Phase 4 workstream. That work should be
tracked explicitly rather than hidden inside the production-trace flywheel.

## Non-Goals

- Rewriting the whole runner or search loop.
- Changing scorer semantics, answer keys, or arena versions without the grader
  version-bump discipline.
- Bypassing G1-G5 approval gates or making deployment decisions.

## Oracle

- [x] The canonical schema validators for task specs, arena metadata, run
      records/receipts, and launch contracts live in Rust and are exercised by
      `bin/gate`. — `crates/daedalus-core/src/validate.rs` is the kernel: one
      `ValidationError`, the canonical TOML + JSON `require_*` families, and the
      `SchemaVersion` registry. `launch.rs` and `cerberus_lab.rs` route their
      require-families through it (private duplicates deleted); `cerberus.rs`
      consumes the `SchemaVersion` registry. The kernel is exercised by the gate
      through `cargo test --workspace` (which `bin/gate` runs) — specifically the
      `launch_contracts_pass_on_accepted_records` unit test, which loads the real
      committed `deliveries/*/contract.toml` records through the kernel. The gate
      is NOT coupled to a `daedalus doctor` invocation (that would make the gate's
      verdict a function of working-tree run litter via `check_run_artifacts`'
      `git status` path, not the code).
- [x] `daedalus doctor` or an equivalent command reports stale, malformed, or
      incompatible receipts/contracts with actionable errors. — new
      `doctor::check_launch_contracts` walks `deliveries/*/contract.toml` through
      `launch::load_contract` and fails with the kernel's actionable message
      (e.g. `contract must be version 1`). It is available for operator use via
      `daedalus doctor`, but not wired into `bin/gate`.
- [x] Existing accepted `pr-review-v2` and `launch-contract-v0` records pass
      the Rust validators without lossy compatibility shims. — the
      `launch_contracts_pass_on_accepted_records` unit test asserts the glob
      discovers `deliveries/launch-contract/contract.toml` and
      `deliveries/pr-review/contract.toml` and loads each through
      `launch::load_contract` with no shim; `daedalus doctor` reports
      `launch-contracts | ok` on the same records for operators.
- [x] New or changed docs for the kernel name the Rust-owned validation
      surfaces; repo-wide Python-era doc cleanup remains owned by
      `backlog.d/042-purge-stale-python-refs.md`. — the `validate.rs` module doc
      names the kernel surfaces; broad Python-prose cleanup stays with 042.
- [x] `bin/gate` green. — fmt + `cargo test --workspace` (all green, including
      the kernel-exercise tests) + clippy `-D warnings`. Deterministic: the gate
      does not depend on `runs/` working-tree state.

## Notes

Pairs with `backlog.d/042-purge-stale-python-refs.md`; do not let doc cleanup
claim the kernel is done. This ticket owns the executable validation kernel,
not the prose migration.

Gate wiring: the kernel is exercised by `bin/gate` via `cargo test --workspace`
(the `launch_contracts_pass_on_accepted_records` test reads the real committed
records through the kernel), with zero coupling to working-tree `runs/` litter.
`check_launch_contracts` remains available in `doctor` for operator use but is
deliberately NOT appended to `bin/gate` — doing so would make the gate's verdict
depend on `git status` of uncommitted run output (`check_run_artifacts`), not on
the code under review.

Deferred: a dedicated `daedalus validate` CLI subcommand, deliberately leaving
`crates/daedalus-cli/src/main.rs` untouched to avoid colliding with concurrent
branches that touch it; the standalone subcommand can land later without
re-doing the kernel. Doc-prose migration (Python-era references) remains owned
by 042, not this ticket.
