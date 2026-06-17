# PR Context: credential-duplicate

The PR switches a helper from environment-based GitHub authentication to an explicit argument list around `gh api`. The review swarm is expected to collapse duplicate credential reports and suppress stale process-startup concerns.

Relevant taxonomy pressure: credential exposure is security-owned and blocking when a reachable token can land in argv, logs, traces, or untrusted output. General-member duplicates should not downgrade the security-owned category.
