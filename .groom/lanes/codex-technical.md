    12	}
arenas/pr-review-v0/tasks/rs-retry-backoff/tests/expected.json
     1	{
     2	  "defects": [
     3	    {
     4	      "id": "off-by-one-attempts",
     5	      "file": "src/retry.rs",
     6	      "line_start": 14,
     7	      "line_end": 17,
     8	      "category": "correctness",
     9	      "note": "Documented as 'up to max_attempts times', but op() runs max_attempts + 1 times: the attempt counter is incremented after the call and only checked with > before retrying."
    10	    },
    11	    {
    12	      "id": "unbounded-backoff-overflow",
    13	      "file": "src/retry.rs",
    14	      "line_start": 18,
    15	      "line_end": 19,
    16	      "category": "correctness",
    17	      "note": "100 * 2u64.pow(attempt) has no cap: for large attempt counts the multiplication overflows (panics in debug builds) and the sleep grows without bound."
    18	    }
    19	  ]
    20	}

codex
## Findings

1. **Candidate identity is too shallow for agent-vs-agent composition.** Evidence: [DESIGN.md](/Users/phaedrus/Development/daedalus/DESIGN.md:197) says candidates should be typed compositions with slots through planner/executor/critic, budgets, artifacts, and output schema, but [runner/run.py](/Users/phaedrus/Development/daedalus/runner/run.py:222) reduces execution to one `kind -> function`, and [DESIGN.md](/Users/phaedrus/Development/daedalus/DESIGN.md:99) defines a flat manifest. Impact: the first MVP break is attribution. N compositions x M trials will not know whether a win came from model, tool policy, prompt packet, critic topology, retry policy, or harness behavior.

2. **The arena contract is still final-answer matching, not work evaluation.** Evidence: [runner/score.py](/Users/phaedrus/Development/daedalus/runner/score.py:40) matches only `file + category + line range`, then collapses to one scalar at [runner/score.py](/Users/phaedrus/Development/daedalus/runner/score.py:66). Every task instruction demands only `findings.json` ([arenas/pr-review-v0/tasks/py-auth-sqli/instruction.md](/Users/phaedrus/Development/daedalus/arenas/pr-review-v0/tasks/py-auth-sqli/instruction.md:13)). Impact: spec-reading, codebase-context PR review, and run-the-app visual QA will reward plausible localized JSON, not whether the agent read the right context, launched the app, inspected screenshots, or changed world state correctly.

3. **`runner/run.py` will break first at orchestration and evidence retention.** Evidence: trials run serially in one nested loop ([runner/run.py](/Users/phaedrus/Development/daedalus/runner/run.py:257)), each workspace is deleted immediately ([runner/run.py](/Users/phaedrus/Development/daedalus/runner/run.py:324)), and the only built-in aggregate is mean reward plus total cost ([runner/run.py](/Users/phaedrus/Development/daedalus/runner/run.py:337)). Pi version is explicitly manual debt in [candidates/pi-kimi.toml](/Users/phaedrus/Development/daedalus/candidates/pi-kimi.toml:4). Impact: harder arenas need resumable runs, parallel scheduling, retained artifacts/transcripts/screenshots, manifest hashes, harness versions, and per-composition distribution summaries.

## Delete/simplify

- Delete `openrouter` as a first-class competitor path once MVP target is composition-vs-composition; keep one-shot only as a baseline adapter, not a peer abstraction.
- Collapse duplicated task instructions into an arena-level instruction template plus per-task intent; six copies make versioning brittle.
- Stop extending ad hoc `KINDS`; replace it with one composition executor boundary before adding more harnesses.
- Do not extend the current scalar scorer into visual/spec/codebase QA. Add scorer families with typed evidence instead.

## Single highest-leverage ticket

**Ticket: Introduce `composition.v1` + experiment-run contract before new arenas.**

Goal: represent a candidate as a versioned composition graph with manifest hash, harness/tool/prompt/model slots, artifact requirements, trial count, budget, and retained evidence paths.

Oracle: running `oracle`, `null`, `baseline`, and `pi-kimi` for `--trials 2` produces a run directory containing immutable composition manifests, per-trial JSONL, retained work artifacts or transcript refs, summary distributions, and a gate test proving scorer fixtures cannot be read from the candidate workspace.
tokens used
57,607
## Findings

1. **Candidate identity is too shallow for agent-vs-agent composition.** Evidence: [DESIGN.md](/Users/phaedrus/Development/daedalus/DESIGN.md:197) says candidates should be typed compositions with slots through planner/executor/critic, budgets, artifacts, and output schema, but [runner/run.py](/Users/phaedrus/Development/daedalus/runner/run.py:222) reduces execution to one `kind -> function`, and [DESIGN.md](/Users/phaedrus/Development/daedalus/DESIGN.md:99) defines a flat manifest. Impact: the first MVP break is attribution. N compositions x M trials will not know whether a win came from model, tool policy, prompt packet, critic topology, retry policy, or harness behavior.

2. **The arena contract is still final-answer matching, not work evaluation.** Evidence: [runner/score.py](/Users/phaedrus/Development/daedalus/runner/score.py:40) matches only `file + category + line range`, then collapses to one scalar at [runner/score.py](/Users/phaedrus/Development/daedalus/runner/score.py:66). Every task instruction demands only `findings.json` ([arenas/pr-review-v0/tasks/py-auth-sqli/instruction.md](/Users/phaedrus/Development/daedalus/arenas/pr-review-v0/tasks/py-auth-sqli/instruction.md:13)). Impact: spec-reading, codebase-context PR review, and run-the-app visual QA will reward plausible localized JSON, not whether the agent read the right context, launched the app, inspected screenshots, or changed world state correctly.

3. **`runner/run.py` will break first at orchestration and evidence retention.** Evidence: trials run serially in one nested loop ([runner/run.py](/Users/phaedrus/Development/daedalus/runner/run.py:257)), each workspace is deleted immediately ([runner/run.py](/Users/phaedrus/Development/daedalus/runner/run.py:324)), and the only built-in aggregate is mean reward plus total cost ([runner/run.py](/Users/phaedrus/Development/daedalus/runner/run.py:337)). Pi version is explicitly manual debt in [candidates/pi-kimi.toml](/Users/phaedrus/Development/daedalus/candidates/pi-kimi.toml:4). Impact: harder arenas need resumable runs, parallel scheduling, retained artifacts/transcripts/screenshots, manifest hashes, harness versions, and per-composition distribution summaries.

## Delete/simplify

- Delete `openrouter` as a first-class competitor path once MVP target is composition-vs-composition; keep one-shot only as a baseline adapter, not a peer abstraction.
- Collapse duplicated task instructions into an arena-level instruction template plus per-task intent; six copies make versioning brittle.
- Stop extending ad hoc `KINDS`; replace it with one composition executor boundary before adding more harnesses.
- Do not extend the current scalar scorer into visual/spec/codebase QA. Add scorer families with typed evidence instead.

## Single highest-leverage ticket

**Ticket: Introduce `composition.v1` + experiment-run contract before new arenas.**

Goal: represent a candidate as a versioned composition graph with manifest hash, harness/tool/prompt/model slots, artifact requirements, trial count, budget, and retained evidence paths.

Oracle: running `oracle`, `null`, `baseline`, and `pi-kimi` for `--trials 2` produces a run directory containing immutable composition manifests, per-trial JSONL, retained work artifacts or transcript refs, summary distributions, and a gate test proving scorer fixtures cannot be read from the candidate workspace.
