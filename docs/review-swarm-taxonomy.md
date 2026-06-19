# Review Swarm Taxonomy

This charter defines the lens boundaries for the first Daedalus PR-review
swarm. It is intentionally narrower than all possible code review: the first
suite needs categories that can be scored, attributed, and adjudicated without
turning the benchmark into taste.

The machine-readable contract lives in the fenced TOML block below. Keep prose
and TOML in sync; `cargo run --quiet --bin daedalus -- taxonomy-validate` treats the TOML block as the
source of truth.

```toml
schema = "review-swarm-taxonomy.v1"
lenses = ["general", "correctness", "security", "verification", "simplification", "product"]
required_lenses = ["general", "correctness", "security"]
optional_lenses = ["verification", "simplification", "product"]

[severity]
levels = ["blocking", "serious", "minor"]
blocking_rule = "A blocking finding must name a concrete failure: input, path, and consequence. Credential exposure, data loss, auth bypass, and reachable runtime crash are blocking by default."

[[category]]
id = "logic-invariant"
lens = "correctness"
description = "Changed behavior violates a documented or locally inferable invariant."
blocking_rule = "Blocking when the invariant protects data integrity, money, auth, or a normal production path."
allowed_overlaps = ["product", "verification"]

[[category]]
id = "runtime-crash"
lens = "correctness"
description = "Reachable panic, exception, null dereference, or unhandled failure path on realistic input."
blocking_rule = "Blocking when reachable on normal input."
allowed_overlaps = ["security", "verification"]

[[category]]
id = "credential-exposure"
lens = "security"
description = "Secrets, credentials, tokens, or private payloads can reach logs, argv, files, traces, comments, or untrusted output."
blocking_rule = "Always blocking when reachable."
allowed_overlaps = ["correctness", "verification"]

[[category]]
id = "authz-bypass"
lens = "security"
description = "The change lets a caller read, write, trigger, or approve something outside its authority."
blocking_rule = "Always blocking when reachable."
allowed_overlaps = ["product"]

[[category]]
id = "injection"
lens = "security"
description = "User-controlled input can be interpreted as markup, code, shell, query, template, link, or another active syntax in a context that expected inert text."
blocking_rule = "Blocking when reachable through user-controlled or untrusted input."
allowed_overlaps = ["correctness", "product"]

[[category]]
id = "verification-break"
lens = "verification"
description = "The change disables, weakens, bypasses, or misreports the gate that should catch the defect."
blocking_rule = "Blocking when a required gate is disabled, loosened, or made falsely green."
allowed_overlaps = ["correctness", "simplification"]

[[category]]
id = "needless-surface"
lens = "simplification"
description = "New abstraction, wrapper, branch, or config surface increases maintenance cost without buying real capability."
blocking_rule = "Blocking only when the surface weakens a gate or creates a production failure path."
allowed_overlaps = ["product", "verification"]

[[category]]
id = "intent-mismatch"
lens = "product"
description = "The change does not satisfy the stated ticket, acceptance behavior, or operator intent."
blocking_rule = "Blocking when required acceptance behavior is missing or contradicted."
allowed_overlaps = ["correctness"]

[[category]]
id = "cross-lens-recall"
lens = "general"
description = "Broad review finding from the general member. The master dedupes it against specialist-owned categories."
blocking_rule = "The general member may report blocking defects but cannot downgrade a specialist-owned finding."
allowed_overlaps = ["correctness", "security", "verification", "simplification", "product"]

[[overlap]]
id = "credential-runtime-crash"
lenses = ["security", "correctness"]
owner = "security"
rule = "If a credential exposure also causes a crash or process failure, security owns the primary category and correctness may provide corroborating evidence."

[[overlap]]
id = "intent-breaks-invariant"
lenses = ["product", "correctness"]
owner = "product"
rule = "If the implementation is internally consistent but fails the accepted ticket intent, product owns the finding."
```

## Adjudication

Ambiguous fixtures are not guessed into a category after a run. They are either
rewritten before freeze or recorded in the arena adjudication log with the
accepted owner, allowed overlaps, and version bump discipline.

## Master Synthesis Rules

The master reviewer receives member artifacts, not answer-key labels. It may
dedupe, suppress, and calibrate findings, but it must disclose missing required
members and cannot invent a successful full-swarm review when a required member
failed.

## Scoring Severity

When an answer key declares `severity`, a finding must report that severity or
a stricter one to match. A weaker, missing, or unknown severity is treated as
a missed true finding and as an unmatched finding for the false-positive
penalty. This is intentional for the first master arena: under-rating a
blocking credential exposure or reachable crash is unsafe enough to fail the
synthesis contract.
