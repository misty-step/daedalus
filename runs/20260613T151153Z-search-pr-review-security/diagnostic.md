# Diagnostic interrupted run: pr-review-security

Status: interrupted by operator/lead agent. This run is not certification
evidence and must not be used as a recommendation source.

Why it exists:

- The run reached seed evaluation after a valid oracle/null/probe rig pass.
- The optimizer-authored `packets/seed-spec-first.md` was visibly corrupted:
  a short word followed by thousands of repeated punctuation characters.
- That degenerate packet drove timeout-heavy behavior and made the run
  unsuitable as comparative evidence.

Remediation in this branch:

- `runner/prompt_packet.py` adds a shared syntactic sanity guard for prompt
  packets.
- `runner/seed.py` falls back to the declared base packet when a successful
  optimizer response is visibly degenerate.
- `runner/mutate.py` rejects degenerate prompt-packet mutation proposals before
  writing child manifests.
- Regression tests live in `tests/test_seed.py` and `tests/test_mutate.py`.

Use `runs/20260613T153751Z-search-pr-review-security` for the fresh
post-remediation security specialist search evidence.
