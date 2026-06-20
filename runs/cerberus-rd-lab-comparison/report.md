# Cerberus Lab Comparison Report

Scope: fixture-only imported Cerberus artifacts. This is sandbox evidence, not a production default change.

| candidate | substrate | valid | lifecycle | verdict | reward | recall | false positives | cost | duration ms |
|---|---|---:|---|---|---:|---:|---:|---:|---:|
| `omp-fixture-review` | `omp` | `true` | `completed` | `WARN` | `1.0` | `1.0` | `0` | `null` | `1` |
| `opencode-fixture-review` | `opencode` | `true` | `completed` | `WARN` | `1.0` | `1.0` | `0` | `null` | `1` |
| `fixture-self-review` | `fixture` | `true` | `completed` | `WARN` | `1.0` | `1.0` | `0` | `null` | `1` |

## Fixture Ordering

`omp-fixture-review` is first under the fixture-only ordering: valid artifacts first, reward descending, known lower cost, then lower latency. This is not a substrate recommendation; live Cerberus OpenCode/OMP runs and Pi comparability remain required before any sandbox recommendation.

## Evidence

- `omp-fixture-review`: `runs/cerberus-rd-lab-omp/report.md`
- `opencode-fixture-review`: `runs/cerberus-rd-lab-opencode/report.md`
- `fixture-self-review`: `runs/cerberus-rd-lab-fixture/report.md`
