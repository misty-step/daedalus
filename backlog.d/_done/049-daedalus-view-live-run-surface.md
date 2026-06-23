# `daedalus view` — live run surface (watch trials, scores, $ spend stream in)

Priority: P1 · Status: delivered · Estimate: L

> **Delivered 2026-06-22.** `daedalus view <run-dir>` — a dependency-free
> terminal roll-up (per-candidate running mean, trials so far, cumulative known
> spend, last-trial heartbeat) that polls `trials.jsonl` and reprints until
> `loop.json` appears, then shows the authoritative run-total spend. `--once`
> for scripts/CI. Chosen over a server/TUI: the rich surfaces are inherently
> post-run (no certified verdict mid-search), so the live signal is the roll-up;
> `report-html` stays the post-run artifact. Reuses `report::aggregate` (no
> drift). Evidence: `crates/daedalus-core/src/view.rs` (+6 tests), live QA —
> `view --once` on a real run matches `loop.json.spend_known_usd` exactly
> ($1.3002), a simulated streaming run, and a follow-loop that self-terminates
> on completion. `bin/gate` green; fresh-context review SHIP (1 nit fixed).

## Goal
A human can watch a search run **as it happens** — trials streaming in with
running scores, per-candidate progress, and live dollar spend — from the same
`trials.jsonl` the runner appends, with no change to the run loop.

## Why
Split from [[044]], which delivered the post-hoc static `report-html`. The
remaining gap is the *live* one: today a run can only be watched by tailing JSON.
The convergent local-first pattern (Inspect AI) pairs the static bundle with a
live `view` server/TUI; the differentiator here is **live $ spend**, which even
Inspect's TUI doesn't surface. This is a distinct architecture from 044 (a
long-running process that polls/streams), which is why it is its own ticket.

## Oracle
- [x] `daedalus view <run-dir>` (terminal roll-up) starts against a run
      directory and streams trials **as they complete**: running mean reward
      per candidate, trials so far, and cumulative known $ spend. (The "/
      expected" denominator is omitted by design — it moves as the search spawns
      child candidates; trials-so-far is the honest live figure.)
- [x] It reads the live `trials.jsonl` incrementally (poll), reusing the
      `report::aggregate` — **no rewrite of the run loop**, no new
      source of truth (JSONL stays authoritative).
- [x] It works while a `daedalus run` is in flight (new trial rows appear within
      a poll interval) and degrades cleanly when the run finishes (stops on
      `loop.json`) or the file is briefly mid-write (a half-written line is
      skipped).
- [x] Local-first / offline (no network, no new dependency); OTel export stays
      an optional downstream, not core.

## Verification System
- Claim: a human can watch a run progress and see live spend without grepping JSON.
- Falsifier: the view misses trials, shows stale scores, or its spend diverges
  from `loop.json.spend_known_usd` at completion.
- Driver: start `daedalus view` against a fixture run dir while a script appends
  trial rows to `trials.jsonl`; assert the rolled-up scores/spend match a final
  `report-html` over the same dir.
- Grader: an integration test that appends known rows and asserts the streamed
  aggregate equals the batch aggregate; manual: watch a real `run`.

## Notes
Reuse the lab.css console/terminal prototypes
(`docs/daedalus-ui-lab/round-2/console.html`, `terminal.html`) for the surface.
Keep the layered architecture from 044: JSONL truth + a swappable derived viewer.
Pairs with [[044]] (shares the aggregate and design language) and [[039]] (the
running stats). The contamination-advisory + `arena-redteam` span audit that
044's oracle bullet 4 also asks for remain tracked under [[044]] as a small
static-report follow-up, not part of this live surface.
