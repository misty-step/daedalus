# Delivered agent: seed4-qwen3-7-plus-checklist

The current certified Daedalus PR-review delivery for
`specs/pr-review/taskspec.toml` on `arenas/pr-review-v2` v0.2.0. It is
recommended only under the current `threshold-then-cheap` rule because it is
the only Pareto-front candidate certified to n >= 5 on every train and
validation task. It is not a production launch approval.

For the maintained export, trace, launch-pack, approval, and closeout sequence,
use `docs/operator-sop.md`. This delivery file records the historical evidence
for this specific agent.

## Composition

| slot | value | evidence |
|---|---|---|
| harness | `pi` 0.78.1 over OpenRouter | captured by `runs/20260611T173632Z-search-pr-review-v0` |
| model | `qwen/qwen3.7-plus` | seed landscape quality leader before certification |
| prompt packet | checklist stance | `runs/20260611T173632Z-search-pr-review-v0/packets/seed-checklist.md` |
| thinking | `low` | sampled seed slot |
| tools | `read`, `bash`, `edit`, `write` | full tool policy |

Measured composition hash: `4a73f1fd213aa1a5`.

The delivery manifest intentionally preserves the measured run packet path so
`daedalus export` recomputes the recorded hash. `packet.md` is a convenience
copy for review; changing the manifest to point at it is a new composition.

## Evidence

Fresh certification command:

```sh
bin/daedalus run specs/pr-review/taskspec.toml --rng-seed 2806 --budget-usd 8 --max-candidates 6 --trials 1 --certify-top 1 --certify-trials 5 --children-per-gen 2 --optimizer-model moonshotai/kimi-k2.6 --max-errors-per-candidate 1
```

Run packet: `runs/20260611T173632Z-search-pr-review-v0`.

- Rig: oracle 1.0, null 0.20, one-shot probe 0.0; arena not saturated.
- Search spread: real candidates ranged from the timed-out Kimi seed at 0.0
  to uncertified `g2b` at 0.6857 and certified seed4 at 0.5714.
- Cutoff evidence: Kimi K2.6 `trace-callers` hit one 600s timeout on
  `py-markup-escape`; validation was skipped by the recorded candidate cutoff.
- Holdout exposure: `g1a`, `g3b`, and `seed3` were exposed to
  `py-live-lock`, `py-export-clear`, and `py-plugin-cache`; the ledger was
  updated in `arenas/pr-review-v2/holdout-ledger.md`.

Certified seed4 search-split results, n = 5 per train and validation task:

| task | split | mean | observation |
|---|---|---:|---|
| `py-markup-escape` | train | 0.00 | stable miss; category/task calibration required |
| `py-padding-clean` | train | 1.00 | stable clean-trap pass |
| `py-progress-speed` | train | 1.00 | stable pass |
| `py-save-leak` | train | 0.80 | mostly stable with one miss |
| `py-formatter-clean` | validation | 1.00 | stable clean-trap pass; trap may be too easy |
| `py-guess-swallow` | validation | 0.00 | stable miss |
| `py-measure-normalize` | validation | 0.20 | search-phase 1.0 regressed under certification |

Overall certified reward: 0.5714 at $0.0170/trial and 70.7s mean wall in
`report.md`.

## Handoff

Generated artifacts:

- `contract.toml`: launch contract with `g3_signed = false`.
- `persona.md`: Bitter Blossom sprite-shaped prompt body tied to the measured
  composition hash.
- `plane-handoff.md`: current Bitter Blossom and Olympus incumbent comparison
  plus import-shape sketches.
- `plane-incumbents.toml`: read-only baseline facts for BB
  `review-coordinator` and Olympus `charon`.
- `cerberus-reviewer-config.json`: sandbox-only
  `ReviewerConfigPacket.v1` handoff for Cerberus. It embeds one measured
  `pr_review` reviewer over the `pi` / OpenRouter `qwen/qwen3.7-plus`
  composition, includes composition hash `4a73f1fd213aa1a5`, and preserves
  G2 as waived plus G3/G4/G5 as pending.

Control-plane imports remain advisory until ticket 029 and G3/G4/G5 human
approval. G2 acceptance covers internal Daedalus contract discovery only. Any
Bitter Blossom use before stronger calibration must be sandboxed and
experimental, not the primary reviewer, and must preserve no-approve/no-merge/
no-code-edit red lines. Olympus should preserve orchestrator-side JSON
validation and posting.

Cerberus validation receipt, 2026-06-19:

- `cargo run --quiet --bin daedalus -- export-cerberus deliveries/pr-review --spec specs/pr-review/taskspec.toml --out deliveries/pr-review/cerberus-reviewer-config.json`
- From `/Users/phaedrus/Development/cerberus`:
  `cargo run --locked -q -p cerberus-cli -- validate-reviewer-config /Users/phaedrus/Development/daedalus/deliveries/pr-review/cerberus-reviewer-config.json`
- From `/Users/phaedrus/Development/cerberus`:
  `cargo run --locked -q -p cerberus-cli -- import-reviewer-config /Users/phaedrus/Development/daedalus/deliveries/pr-review/cerberus-reviewer-config.json --dry-run --out tmp/daedalus-cerberus-export-2026-06-19/import-report.json`

The Cerberus dry-run accepts the packet for comparison, rejects production
import because the packet is sandbox-only and not approved, and leaves Cerberus
defaults unchanged.

## Residual Risks

- The recommended agent is certified but weak: it fails two new tasks at 0/5
  and one validation task at 1/5.
- Higher apparent Pareto candidates (`g2b`, `g3b`) were not certified on every
  train and validation task, so they cannot replace seed4 without another
  certification pass.
- `py-markup-escape`, `py-guess-swallow`, and `py-measure-normalize` are
  promoted in the run's `arena-findings.md`; G2 waives them for internal
  handoff only, and benchmark-quality publication should wait for follow-up
  calibration.
- `approvals/G2-pr-review-v2.md` is accepted by the human reviewer with
  sandbox-only constraints; G3/G4/G5 remain unsigned.
