# Lab notebook

One entry per run: what was tried, what was learned. lineage.md in each run dir has the full story.

## 20260610T160533Z-search-pr-review-v0

- spec `pr-review-v0` (mode threshold-then-cheap) on arena `pr-review-v2` v0.1.0
- stop: plateau · spend $3.027 · generations 2
- recommended: `g1b-seed1-glm-5-spec-first` (hash 44a9aa47e96933ed, reward 1.0, certified=None)
- full story: 20260610T160533Z-search-pr-review-v0/lineage.md

- post-run certification finding: repro at larger n measured ~0.69 (measure 3/5, live-lock 2/5) vs the in-search 1.000 — winner's curse at n=2/task; certification racing (ticket 020) now gates recommendations.

## 20260611T173632Z-search-pr-review-v0

- spec `pr-review-v0` (mode threshold-then-cheap) on arena `pr-review-v2` v0.2.0
- stop: max-candidates · spend $1.7639 · generations 4
- recommended: `seed4-qwen3-7-plus-checklist` (hash 4a73f1fd213aa1a5, reward 0.5714, certified=True)
- confirmed hypotheses: qwen improved the checklist seed's search reward but raised cost; a stricter gpt-5-mini prompt improved reward/cost point estimate before certification.
- alarm: fp-trap-never-fired — every agent passed clean task py-formatter-clean; the trap may be too easy to discriminate FP discipline
- full story: 20260611T173632Z-search-pr-review-v0/lineage.md

## 20260612T024051Z-search-launch-contract-v0

- spec `launch-contract-v0` (mode threshold-then-cheap) on arena `launch-contract-v0` v0.1.0
- stop: max-candidates · spend $0.5066 · generations 3
- recommended: `g3a-seed2-glm-4-7-flash-spec-first` (hash 11ef90f2168dca9a, reward 0.5, certified=True)
- confirmed hypotheses: The worst-trial transcripts show the agent missing seeded defects and emitting false positives on nuanced appr; The default appended coding assistant prompt promises write/edit tools that are not actually provided, confusi
- full story: 20260612T024051Z-search-launch-contract-v0/lineage.md

## 20260612T153450Z-search-launch-contract-v0

- spec `launch-contract-v0` (mode threshold-then-cheap) on arena `launch-contract-v0` v0.1.0
- stop: max-candidates · spend $0.4947 · generations 3
- recommended: `g2b-g1a-seed2-glm-4-7-flash-spec-first` (hash 7523f6b853908df2, reward 0.72, certified=True)
- confirmed hypotheses: prompt constraints reduced unsigned-gate false positives; upgrading the improved prompt to qwen raised reward at higher cost
- full story: 20260612T153450Z-search-launch-contract-v0/lineage.md

## 20260612T220412Z-search-pr-review-master

- spec `pr-review-master` (mode threshold-then-cheap) on arena `pr-review-master-v0` v0.2.0
- stop: max-candidates · spend $0.529 · generations 0
- recommended: `seed2-qwen3-7-plus-spec-first` (hash 491643a3b1de61e3, reward 1.0, certified=True)
- alarm: fp-trap-never-fired — every agent passed clean task clean-noise; the trap may be too easy to discriminate FP discipline
- full story: 20260612T220412Z-search-pr-review-master/lineage.md

## 20260613T153751Z-search-pr-review-security

- spec `pr-review-security` (mode threshold-then-cheap) on arena `pr-review-security-v0` v0.1.0
- stop: max-candidates · spend $0.3527 · generations 0
- recommended: `seed5-kimi-k2-6-checklist` (hash d112f8dd00b0f84b, reward 0.8333, certified=True)
- caveat: injection repeatability was weak (`py-markup-escape` mean 0.50 across two trials); credential-token holdout was perfect
- full story: 20260613T153751Z-search-pr-review-security/lineage.md

## 20260613T161359Z-search-pr-review-correctness

- spec `pr-review-correctness` (mode threshold-then-cheap) on arena `pr-review-correctness-v0` v0.1.0
- stop: max-candidates · spend $0.6253 · generations 0
- recommended: `seed1-gpt-5-mini-spec-first` (hash f090f8060cf36637, reward 0.5714, certified=True)
- caveat: best bounded baseline is not sandbox-ready; it repeatedly failed `py-padding-clean` and missed several defect tasks
- full story: 20260613T161359Z-search-pr-review-correctness/lineage.md

## 20260613T214006Z-search-pr-review-correctness

- spec `pr-review-correctness` (mode threshold-then-cheap) on arena `pr-review-correctness-v0` v0.2.0
- stop: max-candidates · spend $1.3002 · generations 1
- recommended by runner: `g1a-seed3-qwen3-7-plus-skeptic` (hash 196352774b5cab55, reward 0.5625, certified=True)
- caveat: not sandbox-ready; the certified child had lower quality than the non-certified Qwen seed, missed `py-live-lock`, was unstable on the new runtime-crash task, and showed long-tail latency
- confirmed hypothesis: transplanting Qwen's skeptic prompt onto cheaper GLM reduced cost during the paired generation, but did not preserve enough quality under certification/holdout
- full story: 20260613T214006Z-search-pr-review-correctness/lineage.md

## 20260623T183514Z-search-cerberus-reviewer

- spec `cerberus-reviewer` (mode threshold-then-cheap) on arena `pr-review-v0` v0.3.0
- stop: plateau · spend $2.5224 · generations 2
- recommended: `seed2-kimi-k2-7-code-trace-callers` (hash 1df8c73c5cfbb4db, reward 0.7544, certified=True)
- full story: 20260623T183514Z-search-cerberus-reviewer/lineage.md

## 20260625T161856Z-search-cerberus-reviewer

- spec `cerberus-reviewer` (mode threshold-then-cheap) on arena `pr-review-v0` v0.3.0
- stop: plateau · spend $2.0969 · generations 2
- full story: 20260625T161856Z-search-cerberus-reviewer/lineage.md
