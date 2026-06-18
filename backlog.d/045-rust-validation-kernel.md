# Build the Rust validation kernel for schemas, receipts, and contracts

Priority: P1
Status: pending
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

- [ ] The canonical schema validators for task specs, arena metadata, run
      records/receipts, and launch contracts live in Rust and are exercised by
      `bin/gate`.
- [ ] `daedalus doctor` or an equivalent command reports stale, malformed, or
      incompatible receipts/contracts with actionable errors.
- [ ] Existing accepted `pr-review-v2` and `launch-contract-v0` records pass
      the Rust validators without lossy compatibility shims.
- [ ] New or changed docs for the kernel name the Rust-owned validation
      surfaces; repo-wide Python-era doc cleanup remains owned by
      `backlog.d/042-purge-stale-python-refs.md`.
- [ ] `bin/gate` green.

## Notes

Pairs with `backlog.d/042-purge-stale-python-refs.md`; do not let doc cleanup
claim the kernel is done. This ticket owns the executable validation kernel,
not the prose migration.
