# Spike: Langfuse as the lab trace sink

Priority: P2
Status: pending
Estimate: S

## Goal
Self-hosted Langfuse receives traces from lab runs (pi transcripts, judge calls, loop-driver LLM calls) so experiment debugging stops being raw-JSONL archaeology — and the same sink is ready for production observation in Phase 3.

## Non-Goals
- Production deployment observation (Phase 3)
- Replacing JSONL run records (Langfuse is a view, records stay canonical)

## Oracle
- [ ] docker-compose Langfuse up locally; one full experiment's traces visible with cost per trace
- [ ] ADR: what maps to traces/spans (experiment → trace? trial → trace?), and what stays JSONL-only

## Notes
Roadmap Phase 1 item; framework-comparison research consensus: observability
choice matters more than framework choice.
