# Answer-key adjudications — pr-review-v0

The standing workflow for "the candidate found something the key doesn't
list" (DESIGN.md, Adjudication): a human adjudicates each disputed finding
here, then either **ACCEPT** — extend the key, bump the arena version
(prior cross-version averaging becomes invalid; baselines re-run before any
new comparison) — or **OUT-OF-SCOPE** — record the rationale and leave the
key unchanged. Keys improve instead of silently punishing better reviewers.

| id | date | task | finding | ruling |
|---|---|---|---|---|
| ADJ-1 | 2026-06-10 | py-file-cache | concurrent `set()` writers race on the deterministic temp file | **ACCEPT** → key extended, arena 0.2.0 → 0.3.0 |
| ADJ-2 | 2026-06-10 | py-file-cache | `os.rename` raises on Windows when destination exists | **OUT-OF-SCOPE** |

## ADJ-1 — temp-file write race (ACCEPT)

- **Reported by:** pi-kimi (run `runs/20260609T*`; G2 report observation 1a),
  scored as a false positive at the time.
- **Claim:** `cache.set()` writes every writer's payload to the same
  deterministic `_path(key) + ".tmp"`; two concurrent writers for one key
  interleave writes to that file, and the subsequent rename can publish a
  corrupted/partial JSON payload.
- **Analysis:** correct. The deterministic temp name defeats the purpose of
  the write-then-rename pattern under concurrency; the code is new in this
  PR (cache.py is created by the diff), and `concurrency` is in the task
  taxonomy. The key's author missed it — the reviewer was better than the
  key, which is exactly the failure mode this workflow exists for.
- **Ruling:** ACCEPT. Key gains defect `tmp-write-race`
  (cache.py 23–26, concurrency); oracle solution extended to match.
- **Action:** arena version bumped to 0.3.0; oracle/null rig re-validated
  offline (oracle 1.0, null = clean fraction). Cross-version averaging with
  0.2.x records is invalid; re-run baselines before any new comparison on
  0.3.0.

## ADJ-2 — os.rename vs os.replace portability (OUT-OF-SCOPE)

- **Reported by:** pi-kimi (same run; G2 report observation 1b).
- **Claim:** `os.rename(tmp, _path(key))` raises `FileExistsError` on
  Windows when the destination exists; `os.replace` is the portable atomic
  move, so refreshing an existing cache entry crashes on Windows.
- **Analysis:** factually true about the stdlib, but the fixture declares
  no platform contract: nothing in the synthetic app claims Windows
  support, and the task instruction excludes speculation about
  undocumented requirements. On POSIX the code is correct.
- **Ruling:** OUT-OF-SCOPE for this arena version. A finding is a defect
  only against a contract the workspace actually documents. If a future
  fixture states a platform matrix, this class of finding becomes
  acceptable.
- **Note for arena authors:** when platform behavior is meant to be
  in-scope, say so in the workspace (README/spec), or the key cannot
  fairly reward it.
