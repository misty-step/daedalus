# Review-swarm handoff: pr-review-suite

Lab evidence is not launch approval. G3/G4/G5 still gate deployment,
write authority, and production-data re-ingestion.

## Suite

- Mode: `member-only`
- Required members: `general, correctness, security`
- Optional members: `verification, simplification, product`
- Measured cost: `$0.087`
- Measured wall time: `336.2s`

## Import Boundary

- member agents write artifacts only.
- The master/control plane owns synthesis and any later posting.
- Unsigned use is sandbox-only and non-primary.

## Residual Evidence

- Master replay: `{"composition_hash": "491643a3b1de61e3", "contract": "deliveries/pr-review-swarm/master/contract.toml", "evidence": {"run_dir": "runs/20260612T220412Z-search-pr-review-master", "trials": "runs/20260612T220412Z-search-pr-review-master/trials.jsonl"}, "notes": "Synthetic master baseline only; full-swarm export remains blocked on real-member replay.", "real_member_replay": {"evidence": "evidence/real-member-replay.json", "passed": false}}`
