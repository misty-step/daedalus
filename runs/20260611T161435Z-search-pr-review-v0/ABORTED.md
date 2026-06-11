# Aborted diagnostic run

Command:

```sh
bin/daedalus run specs/pr-review/taskspec.toml --rng-seed 2806 --budget-usd 8 --max-candidates 6 --trials 1 --certify-top 1 --certify-trials 5 --children-per-gen 2 --optimizer-model moonshotai/kimi-k2.6
```

Stopped manually on 2026-06-11 after repeated 600-second candidate timeouts
made the run impractical without a failed-candidate cutoff.

Diagnostic evidence retained:

- Rig passed: oracle 1.0, null 0.20, one-shot probe 0.0 across v0.2.0.
- `seed1-gpt-5-mini-checklist` completed train+validation at 6/7 task
  rewards, mean 0.8571, known cost about $0.1385.
- `seed2-kimi-k2-6-trace-callers` completed train+validation at 2/7 task
  rewards, mean 0.2857, with four timeout/error trials.
- `seed3-glm-4-7-flash-test-runner` had already hit a 600-second timeout on
  the first defect task when this run was stopped.

Follow-up: `runner/run.py` and `bin/daedalus` now support
`--max-errors-per-candidate`; the certification run should be restarted with
the same RNG seed and an explicit error cap.
