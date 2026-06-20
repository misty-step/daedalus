# Cerberus Lab Comparison Report

Scope: live imported Cerberus artifacts plus fixture reference. This is sandbox evidence, not a production default change.

| candidate | substrate | valid | lifecycle | verdict | reward | recall | false positives | cost | duration ms |
|---|---|---:|---|---|---:|---:|---:|---:|---:|
| `fixture-self-review` | `fixture` | `true` | `completed` | `WARN` | `1.0` | `1.0` | `0` | `null` | `1` |
| `opencode-live-review` | `opencode` | `true` | `completed` | `WARN` | `0.8` | `1.0` | `1` | `null` | `0` |
| `omp-live-review` | `omp` | `true` | `completed` | `PASS` | `0.0` | `0.0` | `0` | `null` | `0` |

## Ordering

Best live substrate under this fixture objective: `opencode-live-review` on `opencode` with reward `0.8` and cost `null`. The fixture reference remains the oracle ceiling and is not a deployable substrate.

`fixture-self-review` is first under the sandbox ordering: valid artifacts first, reward descending, known lower cost, fixture references after live candidates on ties, then lower latency. Pi is not included because current Pi runs emit Daedalus candidate findings, not Cerberus `ReviewArtifact.v1` lifecycle receipts, so it is incomparable in this adapter proof.

## Evidence

- `fixture-self-review`: `runs/cerberus-rd-lab-fixture/report.md`
- `opencode-live-review`: `runs/cerberus-rd-lab-live-opencode/report.md`
- `omp-live-review`: `runs/cerberus-rd-lab-live-omp/report.md`
