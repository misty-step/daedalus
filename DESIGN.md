# Daedalus Design

Daedalus turns a task specification into a measured, focused agent: spec in,
candidate agent package out, with evidence. This document records the
architecture, the file contracts that are the system's real interfaces, and
the decisions behind them. The README holds the research framing; ROADMAP.md
holds the phases.

## Pipeline

Six stages, connected by file contracts. Daedalus owns stages 1–3 fully and
ships thin reference implementations of 4–5; any control plane (Olympus,
Bitter Blossom) can replace 4–5 by consuming launch contracts and emitting
traces in the agreed shape.

| # | Stage     | Input → Output | Owner |
|---|-----------|----------------|-------|
| 1 | Specify   | conversation → `specs/<id>/taskspec.toml` | Daedalus (master agent interview) |
| 2 | Lab       | taskspec → arena, eval suite, run records, Pareto archive | Daedalus (runner + master agent loop) |
| 3 | Contract  | winning candidate → launch contract + agent package | Daedalus |
| 4 | Deploy    | launch contract → wired trigger + runtime | reference impl (Phase 3); replaceable |
| 5 | Observe   | running agent → traces, regression evals, budget alarms | reference impl (Phase 3); replaceable |
| 6 | Re-iterate| production traces → new fixtures → back to stage 2 | Daedalus (gate G5) |

## Architecture decisions

Decided now:

- **Arena format: Harbor task directories.** `instruction.md`, `environment/`,
  `tests/` (verifier + answer key), `solution/` (oracle), `task.toml`. Phase 0
  runs these locally without containers; Phase 1 adopts Harbor itself
  (`harbor run --agent-import-path …`) and gains Docker isolation, multi-trial
  distributions, and cloud scaling for free. Deviation from pure Harbor in
  Phase 0: the verifier delegates to the shared `runner/score.py` instead of
  being self-contained; containerization in Phase 1 copies the scorer into the
  task image.
- **Candidate harness: pi over OpenRouter, frozen for V1.** The harness is a
  *slot* in the candidate manifest; V1 freezes it to `pi` (minimal system
  prompt, context-engineerable, headless `-p --mode json`, per-message usage
  events). OpenRouter provides one API across model families plus
  per-generation cost. Frozen ≠ hardcoded: Phase 2+ may unfreeze the slot and
  search over harnesses (Harbor already ships claude-code/codex adapters).
- **Master agent: Claude, interactive.** High-judgment, low-volume role.
  Operates via `.agents/skills/daedalus/SKILL.md`. Revisit when the loop goes
  headless.
- **Traces: plain versioned JSONL** (`runs/*.jsonl`). OTel GenAI semantic
  conventions are still in Development status (May 2026); map at export time
  later, never bet the schema on a moving spec.
- **One repo.** Bricks are directories with file contracts between them. Split
  a brick out only when a second external consumer imports it.

Deferred, with reopen triggers:

| Decision | Reopen when |
|---|---|
| Master agent substrate (Claude interactive vs headless pi/SDK) | loop runs unattended |
| Harness slot unfrozen as a search dimension | Phase 2 prompt-mutation plateau |
| GEPA library (vs hand-rolled reflective loop) | hand-rolled reflection stops improving |
| Rust kernel for schemas/validation/contract tooling | schemas stable across two task families |
| Repo split | Olympus/Bitter Blossom imports a brick |

## File contracts

### Task specification — `specs/<id>/taskspec.toml`

Fields: `id`, `goal`, `domain`, `mode` (max-quality | threshold-then-cheap |
fast-enough | pareto | conservative | human-assist), `[inputs]`, `[output]`
contract, `[oracle]`, `[risk]`, `[budget]`, `[trigger]` intent, `[boundaries]`
data limits, `[checkpoints]` gates, `[negative_examples]`. The master agent
derives missing fields through a clarifying interview before any search spend
(gate G1).

### Arena — `arenas/<id>/`

`arena.toml` (id, version, taskspec pointer, frozen-surface note) plus
`tasks/<task-id>/` in Harbor format. Frozen surfaces per version: fixtures,
answer keys, scorer constants, instruction text. Changing any requires a
version bump and re-running all baselines. Candidates never read `tests/` or
`solution/`; the runner copies only `environment/` into the agent workspace.

**Adjudication.** When a candidate reports a finding the key does not list,
a human adjudicates it in `arenas/<id>/adjudications.md` before the next
arena version: ACCEPT (extend the key + oracle solution, bump the version —
cross-version averaging becomes invalid and baselines re-run before any new
comparison) or OUT-OF-SCOPE (record the rationale; the key stands). This is
the eval-improvement flywheel: keys improve instead of silently punishing
reviewers better than their author. Worked example: pr-review-v0 ADJ-1/ADJ-2.
`docs/arena-workbench.md` names the scaffold, freeze validation, disagreement,
and adjudication commands that enforce this flow without changing scorer
constants.

### Findings / answer key (pr-review domain)

Agent output `findings.json`:
`{"findings": [{"file", "line", "category", "description"}]}` with categories
from: `correctness`, `security`, `error-handling`, `concurrency`,
`resource-leak`, `data-loss`.

Answer key `tests/expected.json`:
`{"defects": [{"id", "file", "line_start", "line_end", "category", "note"}]}`.

Scoring families. A task may declare which apply in `tests/scoring.toml`;
absence means deterministic-only (every task to date).

- **deterministic** (`runner/score.py`, primary, always present): a finding
  matches a defect on equal file + category with line inside the range; each
  defect matches once; `reward = max(0, recall − 0.2 × false_positives)`;
  malformed or missing output scores 0; on a clean task any finding scores 0.
  The clean-PR task makes silence a non-strategy and invented findings fatal.
  Trials that error, crash, or trip grader tamper-detection are voided.
- **judge** (`runner/judge.py`, secondary, opt-in): a calibrated 0–5 rubric
  judge for qualities the key can't capture (evidence quality, actionability,
  severity calibration, precision). Rubric files under `rubrics/` are
  versioned and hashed into run records; judge cost is metered into the
  experiment budget. A judge score counts toward keep/discard **only after a
  calibration gate passes**: two independent judge models agree (Spearman ≥
  0.8 on a shared calibration set) AND, where a seeded key exists, the judge
  ranking agrees with the deterministic ranking. This enforces the red line
  "never the only oracle, never trusted without calibration" — an uncalibrated
  judge is reported as a diagnostic, never an objective.

### Candidate manifest — `candidates/<id>.toml` (composition.v1)

Slots: `composition = 1`, `id`, `kind` (null | oracle | oneshot | pi — the
executor; `oneshot` is a saturation probe, a reference like null/oracle,
never a candidate), `model`,
`provider_name`, `prompt_packet` (file reference under `packets/` — the
primary mutable surface), `thinking`, `tools`, `temperature`, `max_tokens`,
`timeout_sec`, `env_allowlist`, optional `provider` table for OpenRouter
routing pinning. The runner computes a **composition hash** over the manifest
plus the resolved packet text, and captures the harness version (`pi
--version`) per run — attribution is mechanical, not remembered. Three
permanent references bound every experiment and are excluded from Pareto
fronts, recommendations, and parent selection: `null` (floor — the arena
can't be passed by silence), `oracle` (ceiling — the verifier works
end-to-end), and the `oneshot` probe (saturation — if one inlined-context
completion rivals the oracle, the arena cannot rank agent configurations and
the search aborts). Run all three after any arena change, before any search
spend.

Experimental discipline: comparisons are always agent vs agent — different
compositions of model, prompt packet, tools, thinking, and hyperparameters.
Two candidates under comparison should differ in as few slots as possible
(ideally one). A one-shot is never a comparison arm; this domain's
deliverable is an agent, so "one-shot vs agent" answers a question nobody is
asking.

### Experiment directory — `runs/<exp-id>/`

- `compositions/<candidate>.json` — immutable snapshot (manifest + hash +
  resolved packet + harness/runner versions)
- `trials.jsonl` — one record per trial, all candidates in the experiment
- `artifacts/<candidate>/<task>-t<n>-<stamp>/` — retained transcripts,
  model responses, findings (gitignored; records reference them)
- `summary.json` — per-candidate, per-task reward distributions
  (rewards list, mean/min/max, wall, cost totals)

Retention rule: run *records* (trials.jsonl, compositions/, summary.json,
rig/seed/loop/pareto JSON, report.md, lineage.md, packets/, manifests/,
artifacts.index) are committed; heavy *artifacts* (transcripts, responses,
findings copies under `artifacts/`) are gitignored local evidence, named per
trial in the committed `artifacts.index` so a record always says what
existed even after a local flush. A multi-candidate experiment's records
stay under ~100KB; artifacts can run to hundreds of MB and never enter git.

Trial record fields: `run_id, ts_start, ts_end, wall_ms, runner_version,
arena_id, arena_version, taskspec, task_id, trial, candidate_id,
candidate_kind, composition_hash, harness_version, model, provider_served,
tokens_prompt, tokens_completion, tokens_cached, cost_usd, reward, recall,
matched, false_positives, expected_defects, findings, artifacts, error,
scorer_error` (+ `agent_exit_code` for CLI candidates). Cost and latency are
part of the objective, not diagnostics; unknown cost is recorded as null,
never guessed. Records are committed; artifacts are local evidence.

### Launch contract — `deliveries/<id>/contract.toml` (contract.v1)

Generated by `bin/daedalus export <delivery> --spec <taskspec>`, never
hand-edited where pinned to evidence. Fields: `contract = 1`, `agent`,
`composition_hash` (binds manifest + resolved packet/skills/agents texts),
`taskspec`, `mode`; `[composition]` (harness + pinned harness_version,
provider, model, thinking, tools, prompt_packet, system_prompt_mode,
timeout); `[trigger]` intent; `[inputs]`/`[output]` contracts;
`[permissions]` (workspace, env allowlist, write actions); `[budgets]`
(cost/wall per run); `[escalation]`; `[observability]` (trace destination,
regression-eval cadence, arena); `[approval]` (`g3_signed = false` until a
human signs `approvals/G3-<agent>.md`). No offline winner deploys without a
signed contract (gate G3).

Alongside it, `persona.md` renders the same composition in the Bitter
Blossom sprite shape (frontmatter name/description/model/skills + the
prompt packet verbatim as the body) so control planes (Olympus, Bitter
Blossom) import the byte-identical system prompt the lab measured; the
embedded `daedalus.composition_hash` ties the persona back to the contract.

`plane-handoff.md` is generated next to the contract and persona. It is the
human-reviewable bridge from lab evidence to control-plane import: composition
hash, prompt packet, model, tools, budgets, output contract, observability,
approval state, residual risks, and per-plane import sketches for Bitter
Blossom task/agent config and Olympus AgentSpec/activation config. When a
delivery directory contains `plane-incumbents.toml`, export includes the
current control-plane baselines as comparison context, but it still does not
mutate those repos or bypass G3/G4/G5.

`bin/daedalus launch-pack <delivery> --plane <bitter-blossom|olympus>` is the
approval-aware import boundary. With an unsigned contract it refuses by
default; `--dry-run` emits only a sandbox packet marked non-deployable,
non-primary-reviewer, and blocked on G3. `bin/daedalus regression <delivery>
--spec <taskspec>` replays the delivery against the arena holdout and writes a
trace view with `runner/trace.py`; `--dry-run` writes the exact replay command
without spending model budget. `bin/daedalus trace <run-dir>` renders existing
committed run records as `trace.otel.json`.

## Human checkpoints

| Gate | What a human approves | Artifact |
|---|---|---|
| G1 | task spec, before search spend | `approvals/G1-<spec>.md` |
| G2 | eval quality (meta-eval report), before scores are trusted | `approvals/G2-<arena>.md` |
| G3 | launch contract, before any deployment | `approvals/G3-<agent>.md` |
| G4 | new production write authority | contract revision |
| G5 | production data flowing back into the lab (redaction reviewed) | `approvals/G5-<run>.md` |

Automation shrinks the *effort* at a gate, never deletes the gate.

## Meta-eval checklist (gate G2)

Before trusting an arena+scorer: oracle scores 1.0; null scores ≈ the
clean-task fraction; the one-shot probe does not saturate the benchmark
(probe mean ≥ oracle − 0.1 aborts the search by default); the
clean task penalizes invented findings; known-bad outputs (style nitpicks,
findings without line numbers, findings on untouched code) score 0; for
LLM-judge scorers (none yet): two independent judges agree and a human-labeled
holdout agrees with the automated score.

## Security posture

Phase 0 candidates run in throwaway temp dirs with the user's local
permissions — acceptable only because fixtures are synthetic and candidates
are our own compositions. The local runner now enforces that boundary:
missing `[risk]` metadata, metadata outside `class = "low"`, or metadata with
`needs_network`, `needs_secrets`, `adversarial_fixtures`, or `user_data`
refuses local execution and points the operator to Harbor/Docker. Phase 0
mitigations:
fixture trees are validated (no symlinks) before copy; candidate-visible
instructions, prompt packets, skills, `AGENTS.md` overlays, and environment
files are scanned for absolute paths into hidden `tests/` or `solution/`;
`tests/` and `solution/` are hashed before each trial and re-checked after it,
so grader tampering voids the trial; crashing candidates score 0.

Residual hole: an unsandboxed tool-using candidate can still guess host
absolute paths. Phase 1 (Harbor/Docker) remains the real isolation boundary
and is mandatory for any arena with sensitive data, network access,
adversarial fixtures, secrets, or untrusted candidate compositions. The
current launch-contract validator is Python (`runner/launch.py`); the Rust
validation kernel reopens after schemas survive two accepted task families or
an external control plane consumes contracts as a runtime dependency. See
`docs/security-posture.md` for exact commands.
