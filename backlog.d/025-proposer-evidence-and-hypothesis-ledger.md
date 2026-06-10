# Richer proposer evidence, hypothesis ledger, combination operator

Priority: P1
Status: ready
Estimate: M

## Goal
The mutation step reasons from real failure structure instead of transcript
tails: it sees the scorer's verdict shape (matched/missed/FP counts, never
the key), states a structured predicted effect that gets scored against the
observed outcome, and can transplant a single slot from any archive member
into the parent — so "cheap model + winning packet" is one proposal away.

## Non-Goals
- Multi-slot mutations (single-variable discipline stays)
- Exposing answer-key contents to candidates or the proposer

## Oracle
- [ ] Evidence block includes per-trial matched count, expected count, and
      false_positives from run records (no key text, no fixture-path lists
      beyond what findings already contain)
- [ ] Proposal schema gains `predicted_effect` ({reward: up|hold, cost:
      down|hold|up}); lineage labels each hypothesis confirmed/refuted by
      comparing prediction to measurement
- [ ] Transplant operator: proposal may name a donor candidate + slot; the
      validator confirms the donor value exists in the archive and differs
      from the parent; the child is still a one-slot delta
- [ ] Proposal prompt no longer biases toward prompt_packet; it states the
      mode and asks for the highest-information experiment
- [ ] Offline tests for each; `bin/gate` green

## Notes
Evidence: capstone g2a was an actively harmful packet born from misread
evidence (it suppressed the true normalize finding); and the obvious
transplant — seed2's glm-4.7-flash under seed1's spec-first packet,
potentially 1.000 at $0.009 — was never proposable. The optimizer's
prediction accuracy also becomes a measurable property of optimizer models.
