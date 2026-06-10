# Adopt Harbor: containerized trials via a PiAgent adapter

Priority: P1
Status: ready
Estimate: M

## Goal
Trials run in Docker isolation under Harbor with parallelism, closing the read-the-answer-key hole and making n≥5 trial distributions cheap.

## Non-Goals
- Cloud scaling (Daytona/Modal) — local Docker first
- Migrating the loop driver (005) onto Harbor internals; it shells out

## Oracle
- [ ] `PiAgent(BaseInstalledAgent)` installs pinned pi in the task container, runs headless over OpenRouter, parses usage into AgentContext
- [ ] `harbor run -p arenas/pr-review-v0 --agent-import-path ...` reproduces the Phase 0 oracle/null/baseline/pi comparison with n≥5 and per-task reward distributions
- [ ] Verifier is self-contained in the task image (scorer copied in; test.sh writes reward.txt)
- [ ] A hostile probe task that tries `cat tests/expected.json` and `cat ~/.ssh/id_rsa` inside the container fails both

## Notes
Roadmap Phase 1; security lane marks isolation as the prerequisite for any
real-repo arena (ticket 009). Harbor v0.13 confirmed to support custom agents
without forking (--agent-import-path).
