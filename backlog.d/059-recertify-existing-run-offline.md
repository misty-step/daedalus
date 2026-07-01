# Re-certify an existing run dir offline (re-gate without re-spending)

Priority: P2 · Status: pending · Estimate: M

> Surfaced while delivering [[056]]: there is no way to apply a new
> certification/reliability/baseline gate to an *already-run* `runs/<id>/`
> without paying for a fresh search. That blocked a $0 demonstration of the 056
> gate on the real 2026-06-23 cerberus data and will block the same for [[055]].

## Goal
A `threshold recertify <run-dir>` (or `report --rebuild`) subcommand that
re-derives `pareto.json` / `report.md` / `loop.json` from a run's existing
`trials.jsonl` + `summary.json` under new gate knobs (`--reliability-floor`,
`--min-effect`, `--certify-trials`, and later `--baseline incumbent:<hash>`),
spending nothing — so a gate change can be validated against real historical
trials.

## Why
The trials are the expensive part and they are already on disk. Re-scoring is
pure arithmetic over them. Without this, every gate/stat change can only be
demonstrated by a paid re-run (non-deterministic, costs budget) or by unit tests
with hand-entered numbers — neither is a live exercise of the real binary on
real data. This is the verification harness the certification layer is missing.

## Oracle
- [ ] `threshold recertify runs/20260623T183514Z-search-cerberus-reviewer
      --reliability-floor 0.10` reproduces the run's report with seed2-kimi
      demoted, spending $0 (no trials, no network).
- [ ] Re-derived `certified` / `recommendable` / CIs match a fresh run on the
      same trials (determinism check on a tiny fixture run).
- [ ] Refuses or warns if the run dir's arena version / scorer constants differ
      from the current ones (a re-score across a version bump is invalid).

## Verification System
- Claim: gate changes can be validated against real trials at $0.
- Falsifier: re-derived verdict diverges from a paid re-run on identical trials.
- Driver: `recertify` over a committed small fixture run + the cerberus run.
- Grader: byte/stat equality of the re-derived vs freshly-computed verdict.
- Evidence packet: the re-derived report alongside the original.
- Cadence: once to build; then it is the default way to demo any gate change.

## Notes
Keep it honest: re-score only surfaces frozen at run time (trials, rewards,
clusters). Anything that would have changed *which trials ran* (models, prompts,
arena) is out of scope — that needs a real run.
