# Live TUI cockpit — watch a research loop run (with a hypothesis ledger)

Priority: P1 · Status: delivered · Estimate: M

> **Shaped 2026-06-23.** Completes 044's open oracle bullet 3 (the deferred live
> view surface, ex-Child-4). Operator chose: **TUI** transport (not a web
> server), **live-run console** scope, and **incremental hypothesis logging** so
> the tested hypotheses stream live instead of only landing in loop.json at the
> end.
>
> **Delivered 2026-06-23.** Both slices shipped. Slice 1: `run_search` gained a
> defaulted `SearchWorld::record_history` hook (pure loop preserved); the CLI
> world appends each entry to `loop.history.jsonl` — streamed log equals the
> returned history exactly (unit-tested), loop.json unchanged. Slice 2:
> `threshold view` renders the cockpit (spend/cap, headroom rig, leader callout
> with per-trial cost, candidate roll-up, hypotheses panel), reusing
> `report::aggregate` so it never drifts from the static report; rig/hypotheses/
> cap fold into `Snapshot` via `with_*` builders, each degrading gracefully when
> its source is absent; the budget cap is persisted to seed.json at run start.
> Fresh-context thermonuclear review applied (hypotheses now render before the
> first trial; alt-screen dropped for Ctrl-C safety). Evidence:
> `crates/threshold-core/src/view.rs` (19 view tests), `search_loop.rs`
> (streaming-equivalence test), `crates/threshold-cli/src/main.rs`; verified live
> via `threshold view <fixture> --once` (mid-flight / early-window / empty-dir);
> `bin/gate` green.

## Goal
A human can watch a `threshold run` as it happens in a single full-screen
terminal cockpit: the current leader, spend-vs-budget-cap, running candidates
with running scores, the rig/headroom panel, the last-trial heartbeat, **and a
live ledger of the hypotheses the optimizer is testing** (each mutation, its
predicted effect, and kept/discarded verdict) — without grepping JSON.

## Why
`threshold view` (049) already gives a live terminal *roll-up*, and `report-html`
(044) gives the rich *post-run* visual. The gap: (a) it's a flat poll-and-reprint,
not a laid-out cockpit; and (b) the **hypotheses** — the most interesting "what
is the search trying right now" signal — only exist in `loop.json.history` at
run completion, so they can't be watched live. This makes the running loop
legible as a *search*, not just a leaderboard.

## Non-Goals
- A web server / browser dashboard (operator chose TUI; the layered-architecture
  rule keeps viewers swappable — a web view can be a later, separate viewer).
- The rich post-run surfaces (CI forest, coverage heatmap, transcript drill) —
  those are inherently post-run and `report-html` owns them.
- A run-history browser across runs (separate scope).
- Making the TUI authoritative — JSONL stays the source of truth; the TUI is a
  derived, read-only viewer.

## Constraints / Invariants
- JSONL is truth; the viewer is a derived, swappable, offline/local-first layer
  (VISION; `report_html` doc). No network.
- Prefer dependency-free: 049's `view.rs` is already a pure, tested `Snapshot`
  model + an ANSI redraw loop. Extend that idiom (alt-screen + panel layout)
  before reaching for a TUI crate.
- The hypothesis log must not change scorer semantics or the run records — it's
  an additional append-only artifact beside `trials.jsonl`.

## Repo Anchors
- `crates/threshold-core/src/view.rs` — the `Snapshot` model + `render`; the live
  data layer to extend.
- `crates/threshold-cli/src/main.rs` `cmd_view` (~509-544) — the poll/redraw loop.
- `crates/threshold-core/src/search_loop.rs` — where children are proposed +
  evaluated (the `history` entries: child_id/parent_id/slot_changed/hypothesis/
  predicted_effect/improved) — the source of the incremental hypothesis log.
- `docs/threshold-ui-lab/round-2/{terminal.html,console.html}` — the cockpit
  design (single-viewport status strip + leader + headroom + candidates).
- `runs/<id>/{trials.jsonl, rig.json, loop.json}` — the data the cockpit reads.

## Design (two slices)

### Slice 1 — incremental hypothesis log (`search_loop.rs`)
As each child is proposed and evaluated, append its history entry to
`runs/<run>/loop.history.jsonl` (one JSON row: `child_id, parent_id,
slot_changed, value_summary, hypothesis, predicted_effect{reward,cost},
generation, parent_reward_mean, reward_mean, improved`) — the same shape that
already lands in `loop.json.history` at the end, just streamed. `loop.json` keeps
the full `history` (read back from the jsonl, or both written). Pure addition; no
behavior change to the search.
- **Verification:** a unit test that the per-generation append matches the final
  `loop.json.history`; a live `run` leaves a growing `loop.history.jsonl`.

### Slice 2 — the cockpit (`view.rs` + `cmd_view`)
Extend `Snapshot` to carry the rig (`rig.json`) + the latest N hypothesis rows
(tail of `loop.history.jsonl`). Render a full-screen layout (alt-screen
`\x1b[?1049h`, panels via box-drawing, no TUI crate): a status header (run ·
running|complete · trials · **spend $X / cap**), a leader figure, a HEADROOM rig
strip (oracle/null/probe), the candidate table (running mean/cost/trials, leader
accented, refs receding), a **HYPOTHESES panel** (last few: `gNx slot→value ·
predicted reward↑/cost↓ · kept|discarded`), and the last-trial heartbeat. Reuse
`report::aggregate`/`cmp_leaderboard` so numbers never drift. Keep `--once` for
scripts; degrade if a panel's source is absent.
- **Verification:** golden render test over a fixture run dir (the panels +
  hypothesis rows + spend-vs-cap appear); `view` against a live `run` shows
  hypotheses streaming in.

## Oracle
- [ ] `threshold run` writes a growing `runs/<id>/loop.history.jsonl` (one row per
      proposed child) whose union equals `loop.json.history` at completion (unit-tested).
- [ ] `threshold view <run-dir>` renders a full-screen cockpit (status+spend/cap,
      leader, rig strip, candidate table, **hypotheses panel**, heartbeat),
      dependency-free, offline; `--once` snapshots it.
- [ ] Watching a live `run`, hypotheses appear in the panel as the optimizer
      proposes them (not only at completion).
- [ ] Golden render test over a fixture asserts the panels + hypothesis rows +
      spend-vs-cap; existing `view` tests still pass.
- [ ] `bin/gate` passes.

## Verification System
- Claim: an operator can watch a run *as a search* (hypotheses + leader + spend),
  live, in the terminal.
- Falsifier: hypotheses don't appear until the run ends; or the cockpit's
  numbers diverge from `report-html` over the same dir.
- Driver: `threshold view <run-dir>` during a live `threshold run`; the golden
  render test; the history-equivalence unit test.
- Evidence: a screenshot/asciinema of the live cockpit + the committed fixture
  golden.

## Premise Source
This ticket (050) + the 2026-06-23 shaping (operator: TUI transport, live-run
console scope, incremental hypothesis logging) + 044 oracle bullet 3.
