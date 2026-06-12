# Launch Contract Review Task

Review intent: {intent}

You are reviewing a proposed agent launch contract or control-plane import
packet. The workspace contains the contract artifacts after a proposed change.

Write your findings to `findings.json` in the workspace root with exactly this
shape:

```json
{
  "findings": [
    {
      "file": "path/relative/to/workspace/root.toml",
      "line": 42,
      "category": "approval-gate",
      "description": "One or two sentences: what is wrong and why it matters."
    }
  ]
}
```

Rules:

- `category` must be one of: `approval-gate`, `evidence`, `permissions`,
  `observability`, `portability`.
- Report real launch-contract defects only: unsigned deployment, missing
  evidence pointers, write authority before approval, trace/observability
  gaps, or non-portable import references.
- Do NOT report style, wording, formatting, naming, or missing production
  ambition.
- One finding per distinct defect.
- If the packet is sound for its stated mode, write `{"findings": []}`.
