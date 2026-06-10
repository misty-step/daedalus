# Code Review Task

PR intent: {intent}

You are reviewing a pull request against the `billing` service. The workspace
contains the repository *after* the change, plus `PR.diff` (the unified diff,
`a/` = old, `b/` = new). The repository includes `README.md` and `SPEC.md`
documenting the billing invariants, and several modules under `billing/` — the
defect may only be visible when the change is read against code or specs in
*other* files, so read the relevant context, not just the diff.

Review ONLY the change introduced in `PR.diff`. Do not report issues in
pre-existing code the diff does not touch.

Write your findings to `findings.json` in the workspace root with exactly this
shape:

```json
{
  "findings": [
    {
      "file": "billing/core/order.py",
      "line": 14,
      "category": "correctness",
      "description": "One or two sentences: what is wrong and why it matters."
    }
  ]
}
```

Rules:

- `category` must be one of: `correctness`, `security`, `error-handling`,
  `concurrency`, `resource-leak`, `data-loss`.
- `line` is the line number in the post-change file where the defect lives.
- Report real defects only, including violations of documented invariants in
  README.md or SPEC.md.
- Do NOT report style, naming, formatting, or performance issues.
- One finding per distinct defect.
- If the change is sound, write `{"findings": []}`. A clean verdict on a clean
  change scores perfectly; invented findings are penalized.
