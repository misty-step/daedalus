# Cerberus Fixture Review Task

The workspace contains a synthetic Rust diff used by Cerberus fixture review
verification.

Write `findings.json` in the workspace root with exactly this shape:

```json
{
  "findings": [
    {
      "file": "path/relative/to/workspace/root.rs",
      "line": 42,
      "category": "correctness",
      "severity": "serious",
      "description": "One or two sentences naming the defect and why it matters."
    }
  ]
}
```

If the change is sound, write `{"findings": []}`.
