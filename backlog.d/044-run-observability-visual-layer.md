# Run-observability + visual review layer (watch runs, sanity-check design/results)

Priority: P1 · Status: in-progress · Estimate: XL

> **Delivered 2026-06-21 — static `report-html` (children 1, 2, 3, 5).**
> `daedalus report-html <run-dir>` emits a self-contained offline HTML report
> (leaderboard, reward-delta CI forest, coverage heatmap, transcript drill, rig
> panel) in the lab.css language, auto-emitted by `run` Stage 5. Evidence:
> `crates/daedalus-core/src/report_html/` (+5 golden/DOM/escaping/anchor
> tests), sample at `runs/20260613T214006Z-search-pr-review-correctness/report.html`,
> `bin/gate` green. Cross-model review caught and fixed a drill-anchor slug
> collision (now keyed by position index).
>
> **Delivered 2026-06-23 — sanity affordances complete (oracle bullet 4).**
> The report's `ARENA SANITY` section now surfaces, beside the rig panel, the
> **contamination advisory** (public-derived → score-inflation risk vs
> contamination-resistant → synthetic/private, with the record's note) and the
> **`arena-redteam` span audit** (count of gameable wide answer-key spans, with
> the flagged tasks and the threshold). Derived from `arenas/<arena_id>` (CWD
> first, then the run-dir's repo root) by the IO shell, reusing
> `workbench::load_contamination` and `score::redteam_audit` — `build_html_from`
> stays disk-free and unit-testable. Degrades gracefully with an honest "arena
> not reachable" note when the arena dir is absent. Evidence:
> `crates/daedalus-core/src/report_html/{mod.rs,render.rs}` (+3 tests: reachable
> verdict+span-count, unreachable note, repo-root resolution), `bin/gate` green.
>
> **Remaining:** only child 4 — the live `daedalus view` server (oracle bullet
> 3) — is split to its own ticket (distinct streaming architecture). 044 stays
> open on that single thread.

## Goal
A human can watch a search run live, sanity-check its design and execution, and review results — which configs win under what conditions, with confidence — through reviewable **visuals**, while everything stays CLI/agent-friendly and local-first.

## Why
The foundry already emits the hard part — a structured source of truth (`loop.json`, `trials.jsonl`, `lineage.md`) and a statistical layer (cluster-robust CIs, pass^k, power, basin) that the entire commercial eval tier *lacks*. What's missing is the **visual skin**: today `report.rs` is markdown-only, and the `docs/daedalus-ui-lab/round-2/` prototypes (atlas / trial / comparison / gates) are unwired mockups. A run can only be inspected by grepping JSON. "Tests pass" ≠ a human can see whether a certified winner actually reviewed code well or gamed the scorer.

External grounding (research 2026-06-18): the convergent local-first pattern is **Inspect AI** (UK AISI) — JSONL source of truth → self-contained static HTML (`inspect view bundle`) → live `view` server/TUI → one-click drill into a trial transcript. Every commercial tool (LangSmith, Braintrust, W&B Weave, Langfuse) **omits rendered confidence intervals** — daedalus's CI/consistency/power layer is the differentiation; it just needs to be *drawn*. Caution (Bowyer et al., ICML 2025, arXiv:2503.01747): at small per-config n, naive CLT bars lie — render the t-corrected interval we already compute and flag when n is too small.

## Oracle
- [x] `daedalus report-html <run-dir>` emits a **self-contained** static HTML (CSS/JS/images base64-inlined, opens from `file://`, PR-attachable, offline) from `loop.json` + `trials.jsonl` — the visual companion to `report.md`, in the Misty Step / lab.css design language.
- [x] It renders the four review surfaces: (a) a **leaderboard** (config × arena, sortable, cost/latency columns); (b) a **CI forest/caterpillar plot** of each certified candidate's reward-delta CI with the `sig`/`clstr→95%` columns (the 039 stats, drawn); (c) a **per-task/per-cluster heatmap** (config × task) to expose Simpson's-paradox wins; (d) one-click **drill from a score row into the trial transcript** (the candidate's findings + the scorer's matched/missed/FP explanation).
- [ ] A **live** surface: `daedalus view <run-dir>` (local server or TUI) streams trials as they complete with running scores, per-candidate progress, and **live $ spend** (the gap even Inspect's TUI doesn't nail) — reads the same JSONL, no rewrite.
- [x] Sanity-check affordances: the rig panel (oracle 1.0 / null floor / probe verdict incl. the slice-B Inconclusive state), the contamination advisory, and the `arena-redteam` span audit are visible in the report so design flaws are caught *before* trusting a ranking. (rig panel: 2026-06-21; contamination advisory + redteam span audit: 2026-06-23 — `report_html::render::sanity_section`.)

## Verification System
- Claim: a human can watch a run and review its results/validity from visuals alone, and an agent can still consume the JSONL.
- Falsifier: a reviewer cannot tell from the report whether a certified win is real (no CI shown), or cannot reach the transcript behind a score.
- Driver: `daedalus report-html` over a real run dir + opening it; `daedalus view` during a live `daedalus run`.
- Grader: the generated HTML renders the four surfaces offline from `file://`; a golden-DOM/snapshot test over a fixture run dir asserts the CI table + heatmap + drill links exist.
- Evidence packet: the committed static HTML beside the run + a screenshot in the PR.
- Cadence: every `run` (report-html auto-generated alongside report.md); `view` on demand.

## Children
1. `daedalus report-html <run-dir>` — static self-contained HTML from `loop.json`/`trials.jsonl`, leaderboard + the existing lab.css aesthetic. (First slice — highest value; wires the round-2 `comparison.html`/`trial.html` prototypes to real data.)
2. Draw the **statistics**: CI forest plot + per-cluster heatmap + the power/consistency columns (the differentiator; render the t-corrected CI from `stats`, flag small-n).
3. **Transcript drill**: from a score cell into the candidate's findings + scorer explanation (matched / missed / false-positive) — the sanity-check affordance.
4. `daedalus view` live surface — incremental metric roll-up + per-candidate progress + live $ spend (local server or TUI), reading the live `trials.jsonl`.
5. Auto-emit `report.html` in `run` Stage 5 beside `report.md`; attach to the G1–G5 gate evidence.

## Notes
Do NOT make a UI the source of truth (research: layered architecture is the convergent design — JSONL truth + swappable derived viewers). Keep it local-first/offline (air-gap-safe); treat any OTel export as an optional downstream (child, not core; OTel GenAI conventions were pre-stable as of mid-2026). Reuse `docs/daedalus-ui-lab/round-2/` (atlas/trial/comparison/gates) as the design — it is the "instrument, not admin panel" direction already prototyped. Pairs with [[039]] (the stats it draws) and [[040]] (the validity signals it surfaces).
