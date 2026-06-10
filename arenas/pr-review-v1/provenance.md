# pr-review-v1 provenance and freeze gate

## Source

Synthetic `billing` service authored for this arena (not a third-party repo),
so there is no upstream license or proprietary code to redact. The snapshot is
multi-file by design: the reviewed change lives in `billing/core/order.py`,
but the invariant it violates is documented in `billing/core/discount.py` and
`SPEC.md` — finding the defect requires cross-file reading.

## Data boundary (G1 / security lane G0.5)

- No real repository, customer data, or credentials. `gitleaks detect` over
  each task's `environment/` must report zero findings before freeze (recorded
  below).
- Candidates never read `tests/` or `solution/` (enforced by the runner).

## Freeze gate (ticket 009 oracle)

Each defective task must show the cheap one-shot baseline scoring < 0.5 while
the oracle scores 1.0 — the evidence that the task genuinely requires context
rather than diff-only pattern matching. Record per-task results here before
treating the arena version as frozen.

| task | oracle | one-shot baseline | requires-context proven |
|---|---|---|---|
| discount-after-tax | 1.0 | **1.0** ($0.0148, 80s) | **NO** |
| extract-subtotal (clean) | 1.0 | n/a | n/a (FP trap) |

**The freeze gate failed, and that is the finding.** The one-shot baseline
scored a perfect 1.0 on the cross-file defect, correctly identifying that
`order_total` violates the discount/tax invariant. Reason: the runner's
one-shot adapter inlines *every* workspace file (including `SPEC.md` and
`discount.py`) into a single prompt, so at this snapshot size "cross-file"
gives the agentic harness no advantage — everything is already in context.

Conclusion: small synthetic snapshots cannot defeat one-shot. A genuinely
non-one-shot-able arena needs either (a) a repository large enough that
inlining all files exceeds or degrades the context window, or (b) tasks whose
defect only manifests under execution (run-the-app QA, ticket 012). This arena
stays **unfrozen** and is retained as (1) a rig/loop exercise over multi-file
material and (2) the concrete evidence motivating larger-repo or
execution-based arenas. See backlog 015.

## gitleaks

```
gitleaks detect --source arenas/pr-review-v1 --no-git
→ scanned ~10.85 KB, no leaks found (2026-06-09)
```
(Synthetic code; the scan is a standing pre-freeze gate for when real
snapshots are added.)
