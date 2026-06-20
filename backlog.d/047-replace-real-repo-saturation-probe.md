# Replace the one-shot saturation probe for real-repo arenas

Priority: P0 · Status: delivered · Estimate: M

## Goal

Make arena freeze validation credible for real-repo-scale arenas whose
candidate-visible workspaces exceed current one-shot model context or trigger
provider empty-output behavior.

## Oracle

- [x] `cargo run --quiet --bin daedalus -- arena-freeze
      arenas/pr-review-correctness-v0 --out-dir <tmp-or-run-dir>` produces a
      non-inconclusive probe verdict for correctness v0.3.0 without running
      candidate search.
- [x] `cargo run --quiet --bin daedalus -- arena-validate
      arenas/pr-review-correctness-v0 --probe-run <freeze-dir>` exits 0 or
      fails only for a true saturated arena, not context overflow or empty
      provider output.
- [x] The replacement probe remains a reference baseline and is excluded from
      Pareto fronts, recommendations, parent selection, and launch exports.
- [x] Docs name the semantics clearly: what failure means, what saturation
      means, and why the probe is comparable across arena versions.
- [x] `bin/gate` passes.

## Verification System

- Claim: a real-repo arena can be frozen before search with a saturation probe
  that is falsifiable and not just an errored model call scored as zero.
- Falsifier: probe trials error on context overflow/empty content while
  validation still passes, or a saturated small arena is allowed through.
- Driver: `arena-freeze`, `arena-validate`, unit tests for probe verdict
  semantics, and at least one correctness v0.3 live freeze receipt.
- Grader: validation report status plus explicit probe error/trial counts.
- Evidence packet: freeze directory with `trials.jsonl`, `summary.json`, and
  `freeze-report.md`; docs/provenance entry naming the command.
- Cadence: before any certified search on a changed arena split or version.

## Notes

Backlog 034 v0.3 rotation fixed the burned holdout, but live freeze attempts
on 2026-06-19 exposed the remaining blocker:

- `deepseek/deepseek-v4-pro` with a 1M context ran five tasks but skipped
  three Pygments tasks by preflight context overflow.
- `meta-llama/llama-4-scout` with a 10M context returned empty content on all
  eight probe trials.

Do not weaken the freeze gate by treating errored probes as unsaturated. The
replacement may be a chunked no-tool probe, a sampled workspace probe with
documented limitations, or a different reference baseline, but it must stay
outside the candidate pool and recommendation logic.

## Delivery Evidence

2026-06-20 delivery uses a bounded review-context one-shot reference probe:
task intent, `PR.diff`, changed files, and small project anchors instead of the
full copied repository. The probe remains `kind = "oneshot"` and therefore
stays reference-only.

- Plan: `docs/047-saturation-probe-plan.html`
- Freeze: `runs/047-saturation-probe-freeze-v3/`
- Report: `runs/047-saturation-probe-freeze-v3/freeze-report.md`
- Offline validation:
  `runs/047-saturation-probe-freeze-v3/arena-validate-report.md`
- Result: oracle `1.0`, null `0.25`, one-shot probe `0.625`, probe errors `0`,
  known probe cost `$0.096589`.
