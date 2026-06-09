# Code Review Task

PR intent: add a pagination helper for list endpoints.

You are reviewing a pull request. The workspace contains the repository files
*after* the change was applied, plus `PR.diff`, the unified diff of the change
(paths are prefixed `a/` for the old version and `b/` for the new version).

Review ONLY the change introduced in `PR.diff`, in the context of the
surrounding code. Do not report issues in pre-existing code that the diff does
not touch.

Write your findings to `findings.json` in the workspace root with exactly this
shape:

```json
{
  "findings": [
    {
      "file": "path/relative/to/workspace/root.py",
      "line": 42,
      "category": "security",
      "description": "One or two sentences: what is wrong and why it matters."
    }
  ]
}
```

Rules:

- `category` must be one of: `correctness`, `security`, `error-handling`,
  `concurrency`, `resource-leak`, `data-loss`.
- `line` is the line number in the post-change file where the defect lives.
- Report real defects only: bugs, vulnerabilities, leaks, races, lost data.
- Do NOT report style, naming, formatting, performance, or documentation
  issues.
- One finding per distinct defect.
- If the change is sound, write `{"findings": []}`. A clean verdict on a clean
  change scores perfectly; invented findings are penalized.
