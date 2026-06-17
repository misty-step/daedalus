# Review Swarm Master Task

PR intent: {intent}

You are the master reviewer. The workspace contains synthetic PR context and
`member_artifacts.json`, a set of specialist review outputs. Your job is to
produce one consolidated review artifact.

Write `findings.json` in the workspace root with exactly this shape:

```json
{
  "findings": [
    {
      "file": "src/file.py",
      "line": 42,
      "category": "credential-exposure",
      "severity": "blocking",
      "description": "One or two sentences naming the defect and why it matters."
    }
  ]
}
```

Rules:

- `category` must be one of the category ids in `docs/review-swarm-taxonomy.md`.
- `severity` must be one of `blocking`, `serious`, or `minor`.
- Keep real defects supported by member evidence.
- Collapse duplicate member reports for the same defect into one finding.
- Suppress member false positives and out-of-scope reports.
- Preserve the stricter justified severity when members disagree.
- If the change is sound, write `{"findings": []}`.
- Do not post comments, edit code, or infer hidden answer-key labels.
