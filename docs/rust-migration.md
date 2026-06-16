# Daedalus → Rust migration

**Goal:** fully migrate Daedalus from Python (`runner/` + `bin/daedalus`) to Rust.

**Why now (decision provenance).** The Python implementation was always an
explicit, time-boxed exception, not drift: `README.md` §"Implementation Biases
if This Becomes Code" sets Rust as the default *if* the repo becomes durable
software, with "Python if a chosen eval framework requires it" as the named
carve-out. Backlog `033` deferred the Rust kernel behind a written reopen
trigger: *"Revisit the Rust-kernel trigger once the second task family lands"*
and *"a Rust contract/schema validator … after two task families."* Both
conditions now hold — four task families exist as arenas
(`pr-review`, `-correctness`, `-security`, `-master`, plus `launch-contract`),
and the Python surface grew from a "thin Phase 0 runner" to ~5,450 LOC. The
schemas the deferral was waiting on have stabilized across families.

## Strategy

Incremental **parity port**, never a blind rewrite:

1. **The file formats are the seams.** Daedalus modules communicate through
   TOML specs/contracts and JSONL run records, not in-process objects. That
   means modules can be ported one at a time, each reading/writing the exact
   same on-disk formats, while the rest stays Python.
2. **Leaf-first.** Port deterministic, well-tested leaf modules before the
   orchestration that composes them; port the `pi`/OpenRouter I/O boundary
   last, behind a trait.
3. **Parity oracle per module.** Every port ships with a test that runs the
   original Python and the Rust port over identical fixtures and asserts the
   verdicts agree (error *text* is impl-defined → compare error *presence*;
   every other field exact). This is the verification loop, instantiated per
   module. Pattern: `crates/daedalus-core/tests/parity_*.rs`.
4. **Both gates green throughout.** `bin/gate` (Python: `py_compile` + pytest)
   stays green for un-ported modules; `cargo test` is the Rust gate. They unify
   only when Rust becomes primary and the Python module is deleted.
5. **Delete on parity.** A Python module is removed only once its Rust port is
   parity-verified *and* every caller is ported or bridged. No dead Python left
   behind ("delete before adding").

## Workspace layout

```
Cargo.toml                      # workspace (resolver 2); Cargo.lock committed
crates/
  daedalus-core/                # lib: deterministic kernel (score, taxonomy, …)
    src/<module>.rs
    tests/parity_<module>.rs    # Python-vs-Rust oracle
  daedalus-cli/                 # bin `daedalus`: ports bin/daedalus subcommands
```

Run the Rust gate: `cargo test`. Run the Python gate: `bin/gate`.

## Module DAG & status

Source LOC from the `migrate-daedalus-rust` branch (includes branch-only
`swarm.py`, `taxonomy.py`). Order roughly = migration order.

### Tier 0 — done
| Module | LOC | Status | Notes |
|---|---|---|---|
| `runner/score.py` | 93 | **ported + parity-verified** | `score::score`; CLI `daedalus score`; 17 unit tests + `parity_score.rs`. The grader is gospel — done first. |

### Tier 1 — deterministic leaves (pure data transforms; strong existing tests)
| Module | LOC | Notes |
|---|---|---|
| `runner/taxonomy.py` | 351 | lens/category validation; `taxonomy-validate`. Branch-only. |
| `runner/prompt_packet.py` | 43 | packet assembly. |
| `runner/lineage.py` | 251 | lineage.md / NOTEBOOK rendering. |
| `runner/trace.py` | 95 | export-time OTel view (ADR-002). |
| `runner/report.py` | 230 | experiment comparison report. |
| `runner/doctor.py` | 168 | cold-start checks (shells `pi --version` — thin). |
| `runner/export.py` | 397 | control-plane export → contract.toml + persona.md. |

### Tier 2 — orchestration logic (deterministic given recorded trial data)
| Module | LOC | Notes |
|---|---|---|
| `runner/loop.py` | 258 | search loop: budget/plateau/keep/certification. Parity over recorded `trials.jsonl`. |
| `runner/mutate.py` | 472 | reflective mutation/proposer plumbing. |
| `runner/judge.py` | 170 | calibrated judge family. |
| `runner/seed.py` | 180 | landscape seed sampling. |
| `runner/launch.py` | 304 | launch-pack rendering + dry-run. |
| `runner/swarm.py` | 341 | review-swarm specialist+master. Branch-only. |
| `runner/workbench.py` | 441 | arena authoring/freeze/calibration. |

### Tier 3 — external boundary (non-deterministic I/O; port behind a trait)
| Module | LOC | Notes |
|---|---|---|
| `runner/run.py` | 685 | **the only real I/O surface**: `pi` subprocess + OpenRouter HTTP (`urllib`) + env keys + usage summing. Trait-gate the model call; HTTP via `ureq`/`reqwest`. |
| `runner/port_harbor.py` | 128 | Harbor task-format adapter. |

### Tier 4 — entrypoints
| Module | LOC | Notes |
|---|---|---|
| `bin/daedalus` | 791 | argparse CLI → `daedalus-cli` (introduce `clap` here). |
| `bin/gate` | 17 | becomes `cargo test` (+ transitional pytest until Python is gone). |
| `bin/harbor-run` | 35 | thin Docker launcher; may stay shell (doctrine allows thin launchers). |

## Known parity gaps to revisit (logged, not yet handled)

From the scorer port — exotic `int()`/answer-key cases real fixtures don't hit,
but which a faithful port should eventually cover or explicitly reject:

- Python `int()` accepts underscore digit separators (`int("1_000")==1000`) and
  non-ASCII digits; the Rust `coerce_line` rejects both.
- Python `int(True)==1` (bool line) — handled; `int(7.5)` truncates — handled.
- Duplicate defect `id`s: Python dedups via a dict (`{d["id"]: d}`) while the
  Rust port keeps a `Vec`, so recall denominators could diverge on malformed
  answer keys with duplicate ids. Real keys have unique ids.

## Log

- **2026-06-16** — Branch `migrate-daedalus-rust` off `deliver-034-review-swarm`
  (captures branch-only `swarm.py`/`taxonomy.py`). Stood up the workspace,
  ported `score.py` → `daedalus-core::score` with 17 unit tests + a
  Python-vs-Rust parity oracle, and `daedalus score` CLI. Next: Tier 1 leaves,
  starting with `taxonomy` and `prompt_packet`.
