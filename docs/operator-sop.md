# Daedalus Operator SOP

This is the maintained cold-start path for a Daedalus task family. Lab
evidence proves a composition in an arena; launch approval is a separate human
gate.

## 0. Start Clean

```sh
git status --short --branch --untracked-files=all
bin/gate
cargo run --quiet --bin daedalus -- doctor
```

Resolve any `doctor` failures before spending model budget. Warnings can be
intentional: unsigned G3/G4/G5 means lab evidence exists but launch is blocked;
local `runs/*/artifacts/` means retained raw evidence exists outside committed
run records.

## 1. Specify And G1

Create or update `specs/<id>/taskspec.toml`. It must name the goal, mode,
inputs, output contract, oracle, risk, budget, trigger intent, data
boundaries, human checkpoints, negative examples, and search space.

Human gate:

- G1 spec approval: `approvals/G1-<id>.md`

No paid search runs before G1 is signed.

## 2. Author And Validate The Arena

Use the workbench for task scaffolds and freeze checks:

```sh
cargo run --quiet --bin daedalus -- arena-scaffold arenas/<arena-id> <task-id> \
  --taskspec specs/<id>/taskspec.toml

cargo run --quiet --bin daedalus -- arena-validate arenas/<arena-id> \
  --probe-run runs/<rig-or-search-run> \
  --report runs/<rig-or-search-run>/freeze-report.md
```

The validation report checks fixture symlinks, answer-key shape, oracle 1.0,
null floor, one-shot probe behavior, split membership, and holdout exposure
counts. Human adjudications go through:

```sh
cargo run --quiet --bin daedalus -- arena-adjudicate arenas/<arena-id> \
  --task <task-id> \
  --finding "<summary>" \
  --ruling ACCEPT \
  --rationale "<why>" \
  --new-version <next-version> \
  --baseline-run runs/<post-change-baseline>
```

Human gate:

- G2 eval-quality approval: `approvals/G2-<arena>.md`

Do not trust comparative scores before G2 signs the arena and scorer quality.

## 3. Run Certified Search

Use the task's declared mode and budget. A typical certified run shape is:

```sh
cargo run --quiet --bin daedalus -- run specs/<id>/taskspec.toml \
  --rng-seed <seed> \
  --budget-usd <budget> \
  --max-candidates <n> \
  --trials 1 \
  --certify-top <k> \
  --certify-trials 5 \
  --children-per-gen 2 \
  --optimizer-model <model> \
  --max-errors-per-candidate 1
```

Committed evidence lives under `runs/<exp-id>/`: `trials.jsonl`,
`summary.json`, `report.md`, `pareto.json`, `lineage.md`, compositions, and
artifact indexes. Raw `artifacts/` are local retained evidence and stay
gitignored.

## 4. Export Contract And Trace

Export only evidence-backed candidates:

```sh
cargo run --quiet --bin daedalus -- export deliveries/<id> --spec specs/<id>/taskspec.toml
cargo run --quiet --bin daedalus -- trace --run-dir runs/<exp-id>
cargo run --quiet --bin daedalus -- regression deliveries/<id> --spec specs/<id>/taskspec.toml --dry-run
```

Launch contracts are validated before import packets render:

```sh
cargo run --quiet --bin daedalus -- launch-pack deliveries/<id> --plane bitter-blossom --dry-run
cargo run --quiet --bin daedalus -- launch-pack deliveries/<id> --plane olympus --dry-run
```

Dry-run packets are sandbox-only and non-deployable.

## 5. Launch Gates

Lab evidence is not launch approval.

- G3 launch approval: `approvals/G3-<agent>.md`
- G4 production write authority: `approvals/G4-<agent-or-plane>.md`
- G5 production data re-ingestion: `approvals/G5-<run>.md`

Before G3, Bitter Blossom and Olympus imports are experimental sandbox
artifacts only. Before G4, write authority remains `none`. Before G5,
production traces or data do not flow back into arena fixtures.

## 6. Close Out

```sh
bin/gate
git status --short --branch --untracked-files=all
git rev-list --left-right --count <branch>...origin/<branch>
```

Close backlog items by moving them to `backlog.d/_done/` and committing with a
`Closes-backlog: <id>` trailer. If the work lands on `master`, push and verify:

```sh
git rev-list --left-right --count master...origin/master
```

Do not claim done with dirty visible paths, unpushed commits, unsigned launch
gates, or missing run evidence.
