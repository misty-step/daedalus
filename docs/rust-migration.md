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

## Shared foundation (`daedalus-core`)

- `pycompat` — Python-semantics helpers every port reuses: `round_half_even`
  (bit-exact with CPython `round()`, format-parse based + battery oracle),
  `is_truthy` (Python `bool()`), `py_str` (Python `str()`), `mean`
  (`statistics.mean`, 1-ULP caveat noted).
- `serde_json` is built with `preserve_order` so JSON object keys keep Python
  dict insertion order — artifacts are byte-reproducible.
- Crate deps available to ports: `serde`, `serde_json`, `toml`, `sha2`, `regex`.

## Module DAG & status

Source LOC from the `migrate-daedalus-rust` branch. Layering is by *runner*
dependency (file formats are the seams; orchestration uses dependency
injection). **All 17 runner modules ported + parity-verified** (deterministic
cores), behind **19 parity oracles** + shared `pycompat`/`pyrandom`. Remaining:
the **live execution glue** (run's `run_pi`/`run_oneshot`/`main`, mutate's
`call_optimizer` — subprocess + OpenRouter HTTP, not parity-testable offline),
the **`bin/daedalus` composition root** → `daedalus-cli` (clap), `bin/gate` →
`cargo test`, then **deleting the Python** once the Rust CLI is verified
end-to-end on the no-spend (null/oracle) paths.

### Ported + parity-verified ✅
| Module | LOC | Rust | Parity oracle |
|---|---|---|---|
| `score.py` | 93 | `score` (+ `daedalus score` CLI) | `parity_score.rs` (24 cases) |
| `prompt_packet.py` | 43 | `prompt_packet` | `parity_prompt_packet.rs` (14-case corpus) |
| `trace.py` | 95 | `trace` (+ `daedalus trace` CLI) | `parity_trace.rs` (semantic + BYTE + real capstone) |
| `report.py` | 230 | `report` | `parity_report.rs` (17 cases incl. real capstone) |
| `lineage.py` | 251 | `lineage` | `parity_lineage.rs` (13 cases) |
| `judge.py` | 170 | `judge` | `parity_judge.rs` (7 deterministic fns; `judge_score` unit-tested w/ fake `call`) |
| `taxonomy.py` | 351 | `taxonomy` | `parity_taxonomy.rs` (6 fixtures incl. real taxonomy) |
| `port_harbor.py` | 128 | `port_harbor` | `parity_port_harbor.rs` (byte-identical output trees, real tasks) |
| — | — | `pycompat` | `parity_pycompat.rs` (~600 round() values) |

### Layer 0 — remaining (no runner deps)
| Module | LOC | Notes |
|---|---|---|
| `loop.py` | 258 | search loop (budget/plateau/keep/certification). Pure DI logic. Parity is cross-language-hard (injected callables) → port + port `test_loop.py`, plus a parity harness driving equivalent deterministic injected fns. **Lead-owned / careful lane.** |
| `swarm.py` | 341 | review-swarm specialist+master. `toml`+`json`; `generated` timestamp = `value or now()` → needs `pycompat::utc_now_iso` (parity passes explicit ts). |
| `doctor.py` | 168 | cold-start checks: `toml`, `pi --version` subprocess (thin boundary), `strptime("%Y-%m-%d")`; watch for `date.today()` expiry (inject/parity-exclude). |

### Layer 1 — deps now satisfied (prompt_packet/score/swarm)
| Module | LOC | Notes |
|---|---|---|
| `mutate.py` | 472 | reflective mutation/proposer; → `prompt_packet` ✅. Has injected proposer LLM `call` → parity deterministic parts, unit-test the call path. |
| `workbench.py` | 441 | arena authoring/freeze/calibration; → `score` ✅. CLI + file I/O heavy. |
| `launch.py` | 304 | launch-pack render + dry-run; → `swarm` (do after swarm). |
| `run.py` | 685 | **Tier 3 I/O boundary**: `pi` subprocess + OpenRouter HTTP + env + usage summing; → `score` ✅. Trait-gate the model call; HTTP via `ureq`. **Lead-owned, no live spend** (replay/fixture only). |

### Layer 2 — deps pending
| Module | LOC | Notes |
|---|---|---|
| `seed.py` | 180 | landscape seed sampling; → `mutate`, `prompt_packet` (after mutate). |
| `export.py` | 397 | control-plane export → contract.toml + persona.md; → `run` (after run). `generated` timestamp like swarm. |

### Tier 4 — entrypoints (last)
| Module | LOC | Notes |
|---|---|---|
| `bin/daedalus` | 791 | argparse CLI → `daedalus-cli` (introduce `clap`); composition root that wires every module. |
| `bin/gate` | 17 | becomes `cargo test` (drop pytest once Python is gone). |
| `bin/harbor-run` | 35 | thin Docker launcher; may stay shell (doctrine allows thin launchers). |

## Parallel port mechanism (proven)

Layer-0/1 leaves are ported by parallel isolated **worktree lanes**, one module
each: pre-scaffold `pub mod X;` + placeholder so lanes touch only disjoint files
(`src/X.rs` + `tests/parity_X.rs`), pre-provision shared deps, dispatch with a
self-contained lane card (no chat context), then cherry-pick each commit (clean,
disjoint) and run the unified gate. Validated on 5 modules across 2 batches.

## Known parity gaps (logged, accepted)

- **score**: Python `int()` accepts `_` digit separators and non-ASCII digits
  (Rust `coerce_line` rejects); duplicate defect `id`s dedup differently (real
  keys have unique ids). Real fixtures don't hit these.
- **taxonomy**: malformed-TOML-fence error *text* differs (Python `tomllib` vs
  Rust `toml` crate); parity asserts `ok=false` + non-empty messages only.
- **judge**: `statistics.mean` exact-Fraction vs f64 — agrees on all tested rank
  vectors; revisit only if a future divergence appears.

## Log

- **2026-06-16** — Branch off `deliver-034-review-swarm`. Stood up the workspace;
  ported **score, prompt_packet, trace, report, lineage, judge, taxonomy,
  port_harbor** + `pycompat`, each behind a Python-vs-Rust parity oracle.
  Hardened `round_half_even` to be bit-exact with CPython `round()` (battery
  oracle). Enabled `serde_json` `preserve_order` (byte-reproducible JSON).
  Established + validated the parallel worktree-lane mechanism (2 batches, 5
  modules). Gates green throughout: `cargo test`/`clippy`/`fmt` + `bin/gate` 174.
- **2026-06-16 (cont.)** — Parallel lanes ported **swarm, doctor, workbench**
  (doctor surfaced a latent Python crash on a missing primitives file — the Rust
  port fails gracefully and documents it) and **launch**. Added `pyrandom`, a
  CPython-exact MT19937 (`shuffle`/`getrandbits` parity-verified) needed by the
  loop and seeder. Lead-ported **loop** (`search_loop`) on it: all `test_loop.py`
  scenarios + a live-Python parity harness incl. a multi-parent shuffle
  trajectory. **13/17 done.** Next: lead-owned **run** (the pi/OpenRouter
  boundary, no live spend — replay/fixture parity), then **mutate**, **seed**,
  **export**, then the entrypoints (`bin/daedalus` → `clap`) and deleting Python.
- **2026-06-16 (cont.)** — Added `py_json_dumps` (Python-faithful `json.dumps`:
  ensure_ascii, `', '`/`': '`, sort_keys) for composition-hash parity. Lanes
  ported the deterministic cores of **run** (composition hash matches Python on
  all 4 real candidates; `summarize` matches real `trials.jsonl`), **mutate**,
  **seed** (sampled compositions match Python under shared seeds via PyRandom),
  and **export** (byte-exact contract/persona on real deliveries, reusing
  `run::load_candidate`). **All 17 module cores done; 19 parity oracles green.**
  Next: run's live execution glue + the `bin/daedalus` CLI, then retire Python.
