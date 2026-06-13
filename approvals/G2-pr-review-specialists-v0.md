# G2 - Eval-quality review: pr-review specialist arenas v0

- **Status:** pending human G2 review
- **Prepared:** 2026-06-13, after explicit correctness/security arena splits
- **Arenas:** `arenas/pr-review-correctness-v0` v0.2.0 and
  `arenas/pr-review-security-v0` v0.1.0
- **Freeze runs:**
  `runs/20260613T213700Z-freeze-pr-review-correctness-v020`,
  `runs/20260613T151035Z-freeze-pr-review-correctness-v0`,
  `runs/20260613T151035Z-freeze-pr-review-security-v0`
- **Search runs:**
  `runs/20260613T214006Z-search-pr-review-correctness`,
  `runs/20260613T161359Z-search-pr-review-correctness`,
  `runs/20260613T153751Z-search-pr-review-security`

## Scope

This packet asks whether the first correctness/security specialist arenas are
acceptable as internal Daedalus review-swarm experimentation targets. It does
not ask for public benchmark approval, G3 launch approval, primary-reviewer
approval, or production-data re-ingestion.

## Commands

```sh
bin/daedalus arena-validate arenas/pr-review-correctness-v0 --probe-run runs/20260613T151035Z-freeze-pr-review-correctness-v0 --report runs/20260613T151035Z-freeze-pr-review-correctness-v0/freeze-report.md
bin/daedalus arena-validate arenas/pr-review-security-v0 --probe-run runs/20260613T151035Z-freeze-pr-review-security-v0 --report runs/20260613T151035Z-freeze-pr-review-security-v0/freeze-report.md
bin/daedalus run specs/pr-review-security/taskspec.toml --rng-seed 9 --budget-usd 0.40 --max-candidates 0 --trials 1 --certify-top 1 --certify-trials 2 --children-per-gen 1 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 2
bin/daedalus run specs/pr-review-correctness/taskspec.toml --rng-seed 11 --budget-usd 0.75 --max-candidates 0 --trials 1 --certify-top 1 --certify-trials 2 --children-per-gen 1 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 2
```

## Freeze Gate Evidence

Security v0.1.0:

- oracle mean: `1.0`
- null mean: `0.3333`
- one-shot probe mean: `0.0`
- holdout exposure: `{"py-save-token-leak": 0}` at freeze time
- freeze report: `runs/20260613T151035Z-freeze-pr-review-security-v0/freeze-report.md`

Correctness v0.1.0:

- oracle mean: `1.0`
- null mean: `0.2857`
- one-shot probe mean: `0.0`
- holdout exposure:
  `{"py-export-clear": 0, "py-plugin-cache": 0}` at freeze time
- freeze report: `runs/20260613T151035Z-freeze-pr-review-correctness-v0/freeze-report.md`

Correctness v0.2.0:

- oracle mean: `1.0`
- null mean: `0.25`
- one-shot probe mean: `0.0`
- holdout exposure:
  `{"py-export-clear": 4, "py-plugin-cache": 4}` at freeze time
- freeze report:
  `runs/20260613T213700Z-freeze-pr-review-correctness-v020/freeze-report.md`

## Search Evidence

Security bounded baseline:

- recommended composition: `seed5-kimi-k2-6-checklist`
- model: `moonshotai/kimi-k2.6`
- composition hash: `d112f8dd00b0f84b`
- certified: yes, with repeated train/validation and holdout trials under the
  seed-only run
- mean reward: `0.8333`
- total known spend: `$0.3527`
- caveat: `py-markup-escape` injection repeatability was weak (`0.50` across
  two measured trials), even though the authored token-leak holdout was
  perfect

Correctness bounded baseline:

- recommended composition: `seed1-gpt-5-mini-spec-first`
- model: `openai/gpt-5-mini`
- composition hash: `f090f8060cf36637`
- certified: yes, under the seed-only run shape
- mean reward: `0.5714`
- total known spend: `$0.6253`
- caveat: this is not a strong correctness member. It repeatedly failed the
  `py-padding-clean` false-positive trap and missed multiple defect tasks,
  including `py-export-clear` holdout.

Correctness v0.2 loop:

- arena addition: `py-formatter-missing-crash` for the owned `runtime-crash`
  category
- runner-recommended composition: `g1a-seed3-qwen3-7-plus-skeptic`
- model: `z-ai/glm-4.7-flash`
- composition hash: `196352774b5cab55`
- certified: yes, under this run shape
- mean reward: `0.5625`
- total known spend: `$1.3002`
- caveat: this is a failed quality iteration, not a sandbox member. The
  certified child was unstable on the clean trap and runtime-crash task,
  missed `py-live-lock`, and lost on mean reward to a non-certified Qwen seed.
  The v0.2 holdout is now burned above the default threshold after this search;
  further certified holdout search requires rotation/version bump or an
  explicit diagnostic-only waiver.

## Evidence Artifacts

- `runs/20260613T153751Z-search-pr-review-security/report.md`
- `runs/20260613T153751Z-search-pr-review-security/pareto.json`
- `runs/20260613T153751Z-search-pr-review-security/loop.json`
- `runs/20260613T153751Z-search-pr-review-security/lineage.md`
- `runs/20260613T153751Z-search-pr-review-security/trials.jsonl`
- `runs/20260613T161359Z-search-pr-review-correctness/report.md`
- `runs/20260613T161359Z-search-pr-review-correctness/pareto.json`
- `runs/20260613T161359Z-search-pr-review-correctness/loop.json`
- `runs/20260613T161359Z-search-pr-review-correctness/lineage.md`
- `runs/20260613T161359Z-search-pr-review-correctness/trials.jsonl`
- `runs/20260613T213700Z-freeze-pr-review-correctness-v020/freeze-report.md`
- `runs/20260613T214006Z-search-pr-review-correctness/report.md`
- `runs/20260613T214006Z-search-pr-review-correctness/pareto.json`
- `runs/20260613T214006Z-search-pr-review-correctness/loop.json`
- `runs/20260613T214006Z-search-pr-review-correctness/lineage.md`
- `runs/20260613T214006Z-search-pr-review-correctness/trials.jsonl`
- `runs/20260613T151153Z-search-pr-review-security/diagnostic.md`

## Arena Findings For Human Decision

1. These arenas now have explicit specialist fixture roots, so the correctness
   and security task specs no longer pretend the full `pr-review-v2` arena is
   a runnable lens subset.
2. The one-shot probe remains noisy: every probe attempt returned HTTP 400 and
   known `$0.0000` cost. This prevents saturation but does not demonstrate
   one-shot competence.
3. The security arena has only one adapted injection defect, one authored
   credential-exposure defect, and one clean task. It is adequate for internal
   spread evidence, not public benchmark quality.
4. The correctness arena now covers both owned categories, but no candidate
   approached enterprise-quality review behavior in the bounded v0.1/v0.2
   runs.
5. The first security search attempt found a harness bug: optimizer-authored
   prompt packets could be syntactically corrupted yet still enter the seed
   pool. The current branch adds packet sanity guards and regression tests.
6. Latency is a gating concern. Slow candidates took multi-minute trials on
   small specialist arenas, which does not fit event-plane ergonomics without
   pruning, tighter wall caps, or parallel execution.

## Human Review State

- [ ] Human reviewer accepts these arenas as internal specialist baselines for
      continued Daedalus review-swarm experimentation.
- [ ] Human reviewer accepts security v0.1.0 but requests correctness v0.2.0
      before any suite replay/export.
- [ ] Human reviewer requests both arenas be revised before further suite
      work.

No approval is granted by this packet. G3, G4, and G5 remain unsigned.
