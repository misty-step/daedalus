# `daedalus view` — live run surface (watch trials, scores, $ spend stream in)

Priority: P1 · Status: ready · Estimate: L

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
- [ ] `daedalus view <run-dir>` (local server or TUI) starts against a run
      directory and streams trials **as they complete**: running mean reward
      per candidate, trials done / expected, and cumulative known $ spend.
- [ ] It reads the live `trials.jsonl` incrementally (tail/poll), reusing the
      `report_html`/`report` aggregate — **no rewrite of the run loop**, no new
      source of truth (JSONL stays authoritative).
- [ ] It works while a `daedalus run` is in flight (new trial rows appear within
      a poll interval) and degrades cleanly when the run finishes or the file is
      briefly mid-write.
- [ ] Local-first / offline (no network); an `arena-redteam` / OTel export stays
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
