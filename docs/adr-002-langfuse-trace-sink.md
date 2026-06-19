# ADR-002: Trace sink as an export-time view, Langfuse via OTLP

Status: accepted (2026-06-10) — spike for backlog 014

## Context

Lab runs already emit complete JSONL run records (tokens, cost, latency,
reward, composition hash, transcripts as gitignored artifacts). Debugging a
search run today means raw-JSONL archaeology across `trials.jsonl`,
`loop.json`, and per-trial artifact dirs. A trace sink (Langfuse, or any OTel
backend) would make experiment debugging visual and give cost-per-trace
aggregation for free, and the same sink is the Phase 3 production-observation
substrate.

Two architecture questions: (1) runtime instrumentation vs export-time view;
(2) what maps to traces and spans.

## Decision

**Tracing is an export-time view, not runtime instrumentation.** DESIGN.md
already fixed this direction ("OTel GenAI semantic conventions are still in
Development; map at export time later, never bet the schema on a moving
spec"). Historical pre-migration artifact `runner/trace.py` proved the mapping;
the current implementation is the Rust `trace` module. A thin, separable adapter
can post the derived trace to a sink. The runner stays dependency-light, and the
JSONL records remain canonical — the trace is regenerable at any time from
records that are already committed.

**Mapping (one experiment = one trace; one trial = one span):**

| daedalus | OTel/Langfuse | rationale |
|---|---|---|
| experiment dir (`runs/<id>`) | trace | the unit a human debugs end to end |
| trial (one run record) | span | the unit with a cost, latency, and reward |
| candidate | span attribute + name prefix | grouping, not a nesting level — keeps the tree flat and queryable |
| model / tokens / cost | `gen_ai.*` span attributes | GenAI semantic convention, pinned at export |
| reward / FP / hash / task | `daedalus.*` span attributes | lab-specific, namespaced to avoid colliding with the moving spec |

Optimizer/judge LLM calls become sibling spans under the same trace when
their costs are recorded (future: they currently fold into `optimizer_costs`;
promote to spans when per-call records exist).

## What was and was not executed

- **Built and tested (historical pre-migration spike):** `runner/trace.py` + `tests/test_trace.py`
  (4 tests), validated on the real capstone run — 13 candidates, 76 trials,
  $2.93 total rendered into a trace with per-span reward/cost. The mapping is
  real code, not a diagram.
- **Not stood up in this spike:** a live self-hosted Langfuse container. The
  current Langfuse self-host stack is heavy (Postgres + ClickHouse + Redis +
  S3/MinIO via the official docker-compose); standing it up and wiring OTLP
  ingestion is a half-day of infra that the export-time architecture
  deliberately decouples from the lab. The oracle's "compose up, traces
  visible" item is **deferred**, not faked — and the decoupling is the point:
  the converter is done and the sink is swappable.

## Consequences

- Trace emission can be added behind a `--trace` flag or a `DAEDALUS_OTLP_ENDPOINT`
  env var as a post-stage-5 step (Rust trace export -> OTLP exporter) with no
  change to the search loop.
- Langfuse is the recommended sink (self-hosted, OTLP-native, GenAI-aware,
  cost rollups), but nothing binds us to it — any OTel backend ingests the
  same JSON.
- Revisit the live stand-up when (a) a search run's debugging actually hurts
  at current scale, or (b) Phase 3 production observation needs the sink
  anyway — do it once, for both.
- When OTel GenAI semconv leaves Development, bump `OTEL_GENAI_VERSION` in
  the trace exporter and re-map; records are untouched.
