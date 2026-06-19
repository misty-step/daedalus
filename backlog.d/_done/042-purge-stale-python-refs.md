# Purge stale post-migration command references from docs and CLI output

Priority: P1 · Status: done · Estimate: M

## Goal
No authoritative doc, backlog anchor, or generated CLI output tells a cold
agent or operator to run deleted Python or the retired `bin/daedalus` shim; every
reference points at the Rust binary reality.

## Oracle
- [x] `rg -n 'bin/daedalus|runner/|python3|pytest|\.py\b' README.md DESIGN.md ROADMAP.md docs backlog.d/034-build-daedalus-review-swarm.md crates/daedalus-cli/src/main.rs Cargo.toml crates/daedalus-cli/Cargo.toml | rg -v 'docs/rust-migration.md|historical|Historical|pre-migration'` returns no output.
- [x] `rg -n 'bin/daedalus|runner/|python3|pytest' arenas/*/provenance.md arenas/*/holdout-ledger.md | rg -v 'historical|Historical|pre-migration'` returns no output.
- [x] `rg -n 'python3 runner/run.py|bin/daedalus' runs/*/regression-command.txt` returns no output.
- [x] Operator-facing command paths use the actual Rust binary path (`cargo run --quiet --bin daedalus -- ...` in the checkout, or an installed `daedalus` binary when documented as installed), not the deleted `bin/daedalus` shim.
- [x] DESIGN.md anchors corrected: `runner/score.py|judge.py|trace.py|launch.py` -> `crates/daedalus-core/src/{score,judge,trace,launch}.rs`, and regression dry-run docs no longer claim to write a `runner/trace.py` replay path.
- [x] `docs/operator-sop.md` cold-start and run/export/launch commands work when copied from a fresh checkout.
- [x] `docs/security-posture.md` local runner, Harbor port, launch validator, and verification-command sections point at Rust-era commands and the current isolation boundary.
- [x] `docs/adr-004-review-swarm-contract.md:58` `python3 -m pytest ...` -> `bin/gate` (the canonical gate).
- [x] 034's Repo Anchors and Delivery Progress point at Rust CLI/module equivalents (`daedalus export-suite`, `taxonomy-validate`, `crates/daedalus-core/src/{taxonomy,swarm}.rs`), not `runner/*.py`.
- [x] `daedalus regression <delivery> --spec <taskspec> --dry-run` writes a replay command that invokes the Rust CLI, not `python3 runner/run.py`.
- [x] `cargo run --quiet --bin daedalus -- doctor` and `bin/gate` are green after the cleanup.

## Verification System
- Claim: a cold agent can copy authoritative docs and generated dry-run output and reach the Rust CLI paths that actually exist in this checkout.
- Falsifier: `bin/daedalus doctor` is still presented as copy-pasteable, any live docs tell the user to run a deleted `runner/*.py`, or the regression dry-run emits `python3 runner/run.py`.
- Driver: `rg -n 'bin/daedalus|runner/|python3|pytest|\.py\b' ...`, copied SOP commands through `cargo run --quiet --bin daedalus -- ...`, and a regression dry-run against an existing delivery/spec fixture.
- Grader: every remaining hit is explicitly historical/pre-migration, and the copied commands exit 0 or fail only for named domain prerequisites rather than missing executables.
- Evidence packet: the `rg` transcript, dry-run `regression-command.txt`, `cargo run --quiet --bin daedalus -- doctor`, and `bin/gate`.
- Cadence: after Rust-migration docs edits and before any cold-start/operator SOP claim.

## Notes
Same staleness class the /ci pass already fixed in AGENTS.md, now expanded by
the 2026-06-18 groom. Live evidence: `bin/daedalus doctor` exits 127 because no
`bin/daedalus` exists (`find bin` shows only `bin/gate` and `bin/harbor-run`),
while `cargo run --quiet --bin daedalus -- doctor` succeeds with only expected
approval/artifact warnings. Found refs: `docs/operator-sop.md:12,37,40,50,70,92-101`,
`docs/security-posture.md:5,30,42,55,71,79`, `crates/daedalus-cli/src/main.rs:614`,
`DESIGN.md:34,107,113,211,256`, `docs/adr-004-review-swarm-contract.md:58`,
`ROADMAP.md:108,158-159` (historical phase notes - annotate), historical run
evidence in `docs/review-swarm-vertical-slice.md:61,142-160` (annotate, do not
delete), and `backlog.d/034:110,423,425`. This outranks new observability polish
because it breaks cold-start trust and generated command output, not just prose.

Delivered 2026-06-18 on branch `deliver/042-command-truth`. Evidence:
`cargo test -p daedalus-cli regression_dry_run_writes_rust_cli_replay_command`;
`cargo run --quiet --bin daedalus -- regression deliveries/pr-review --spec
specs/pr-review/taskspec.toml --trials 1 --exp-dir
/tmp/daedalus-042-regression-smoke --dry-run` wrote a Rust replay command;
existing committed `runs/*/regression-command.txt` dry-run receipts were
regenerated with the Rust replay command;
`cargo run --quiet --bin daedalus -- doctor` passed with expected unsigned-G3
and local-artifact warnings; `cargo run --quiet --bin daedalus -- launch-pack
deliveries/pr-review --plane bitter-blossom --dry-run --out-dir
/tmp/daedalus-042-launch-smoke` emitted an import packet; `bin/gate` passed.
`docs/rust-migration.md` remains the historical pre-migration ledger and is
explicitly labelled as non-current command evidence.
