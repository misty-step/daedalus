---
name: threshold-qa
description: |
  QA Threshold changes by exercising a real arena/eval run and reading whether
  the scored output is trustworthy, not just tests. Threshold is an eval foundry
  (Rust kernel + `threshold` CLI over a `pi`/OpenRouter substrate): `bin/gate`
  green proves the machinery, NOT that scores are calibrated or grounded.
  "Tests pass" is not QA. Use when: "QA this", "verify the run", "smoke test
  threshold", "did the scorer change help", "are the scores calibrated", "test
  the arena". Trigger: /threshold-qa.
argument-hint: "[gate|scorer|arena-run|doctor|export|analysis]"
---

# threshold-qa

QA in Threshold means verifying the surface that changed against a real scored
run. `bin/gate` (`cargo fmt --check` + `cargo test --workspace` + `cargo clippy
--all-targets -D warnings`; CI == `bin/gate`, offline, no keys) is the
deterministic gate — **necessary but not sufficient**. Its run-path tests use
fixtures and canned oracle/null candidates, so it CANNOT tell you whether a real
run's scores are calibrated (oracle≈1.0, null at floor) or a candidate's reward
is grounded vs gamed. That is a human read of the freeze report +
`report.md`/`pareto.json` — the point of this skill.

## Surfaces

| Changed area | Surface | QA path |
|---|---|---|
| `crates/threshold-core/src/score.rs`, `arenas/*/tasks/*/tests/expected.json`, `arenas/*/arena.toml` | **The grader (gospel)** | Re-freeze + validate the arena: oracle must hit 1.0, null at floor. Bump arena version; re-run oracle/null. `bin/gate` can't prove calibration. |
| `crates/threshold-core/src/{run,search_loop,mutate,stats}.rs` | Arena/eval runner | Live `run` (or `--estimate` offline first), then **read the scores** in `report.md`/`pareto.json`/`loop.json`, not exit 0 |
| `crates/threshold-core/src/doctor.rs`, `docs/primitives.md`, `specs/*/taskspec.toml` | Cold-start readiness | `threshold doctor` — offline; model-primitives + roster-in-pool |
| `crates/threshold-core/src/{report,report_html,view,compare,lineage,trace}.rs` | Analysis/reporting | Render over an existing run dir; eyeball the artifact |
| `crates/threshold-core/src/{export,launch,cerberus,swarm}.rs` | Export / handoff | `export` + `launch-pack --dry-run` (must stay sandbox/rejected while gates unsigned) |

## Build + offline checks (run first, no spend)

```sh
bin/gate                                                 # offline gate; == CI
cargo build -p threshold-cli                              # → target/debug/threshold
cargo run --quiet --bin threshold -- doctor              # readiness, no model spend
cargo run --quiet --bin threshold -- --help             # full subcommand list
```

- Live runs need `OPENROUTER_API_KEY` in env (read directly in `run.rs`; agent
  env sources it from `op://Agents`) AND the `pi` CLI on PATH (`pi`/`incumbent`
  candidates shell out to it). `oracle`, `null`, `doctor`, and `--estimate` are
  fully offline.
- Data lives in-repo: `arenas/<id>/`, `specs/<id>/taskspec.toml`,
  `candidates/{oracle,null,probe-oneshot,pi-kimi}.toml`, run records in `runs/`.

## Scorer QA — the calibration anchor (do this for ANY grader change)

`AGENTS.md`: **"the grader is gospel."** A change to `score.rs`, an answer key,
or scorer constants requires an arena version bump (`arenas/<id>/arena.toml`)
and re-running the oracle/null baselines. Never average reward across arena
versions.

```sh
cargo run --quiet --bin threshold -- arena-freeze arenas/<id> --out-dir runs/<freeze>
cargo run --quiet --bin threshold -- arena-validate arenas/<id> \
  --probe-run runs/<freeze> --report runs/<freeze>/freeze-report.md
cargo run --quiet --bin threshold -- arena-redteam arenas/<id>   # flag gameable wide spans
```

Then **read `runs/<freeze>/freeze-report.md` by hand** — this is the QA:
1. **Oracle == 1.0** on every task and **null sits at the floor.** If not, the
   scorer is broken and every downstream comparative score is invalid.
2. Answer-key shape, fixture symlinks, split membership, holdout exposure counts
   all pass. `arena-redteam` shows no wide (>8-line) gameable spans, or you've
   tightened/re-baselined them.

## Arena-run QA — is the scored output trustworthy (the real QA)

```sh
# offline forecast first (projects trials + worst-case cost, then exits — no spend)
cargo run --quiet --bin threshold -- run specs/<id>/taskspec.toml --estimate

# live certified search (spends; needs OPENROUTER_API_KEY + pi). Shape per operator-sop §3:
cargo run --quiet --bin threshold -- run specs/<id>/taskspec.toml \
  --rng-seed <seed> --budget-usd <b> --max-candidates <n> --trials 1 \
  --certify-top <k> --certify-trials 5 --reliability-floor <p> \
  --children-per-gen 2 --max-errors-per-candidate 1
```

Then **read the scored output** in `runs/<exp-id>/` — the run exiting 0 is not QA:
1. `report.md` leaderboard: is the ranking sane? Does the oracle ceiling top it
   and the null floor sit at the bottom (calibration held during the real run)?
2. `pareto.json`: the `recommended` candidate's `reward_mean` beats the baseline
   with a `reward_delta_ci` lower bound above `--min-effect` (a delta inside its
   CI is not a result).
3. `loop.json`: `reliability_floor` / `recommendable` / `reliability_demoted` —
   a high mean over a config that fails most of its runs is NOT deployable
   (τ-bench). Confirm cost is real (`spend_known_usd`); unknown cost is `null`,
   never 0.
4. Robustness across seeds: `threshold basin runs/<seedA> runs/<seedB> ...` — a
   BASIN TRAP verdict means the winner is seed-dependent, not real.

## Analysis / export QA (offline, over existing runs)

```sh
cargo run --quiet --bin threshold -- report-html runs/<exp>      # → report.html (open from file://)
cargo run --quiet --bin threshold -- view runs/<exp> --once      # one live roll-up snapshot
cargo run --quiet --bin threshold -- compare runs/<A> runs/<B>   # two-run reward/rank/cost delta
cargo run --quiet --bin threshold -- launch-pack deliveries/<id> --plane olympus --dry-run
```

Confirm `report.html` renders offline; the dry-run launch packet must report
**rejected / sandbox-only** while any G3/G4/G5 approval is unsigned.

## Gotchas

- **`bin/gate` green says nothing about score quality.** A scorer regression or
  ungrounded reviewer passes the whole gate. Freeze-validate any grader change.
- **Grader change without an arena version bump silently corrupts cross-run
  comparison.** Bump `arena.toml`, re-run baselines, never average across versions.
- **Run records are append-only evidence** — never edit/delete committed
  `runs/*.jsonl`. Unknown cost is `null`, never an estimate.
- Live search costs money and needs `pi` + `OPENROUTER_API_KEY`; `--estimate`
  first, don't loop live runs — read the one run dir. Candidates must never read
  `tests/`/`solution/`. G1–G5 are human approvals; dry-run packets never deploy.

## Report

Return: **verdict** (PASS / FAIL / UNVERIFIED) · exact commands run · surfaces
exercised (machinery vs scores) · artifacts inspected (paths under `runs/…`:
freeze-report.md, report.md, pareto.json, loop.json) · for a grader change, the
oracle==1.0 / null-floor calibration read · what was NOT covered (e.g. "gate
only, no live run") and whether a re-freeze or paired-seed run is still owed.
