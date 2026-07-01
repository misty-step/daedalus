# Landmark Fleet Integration Playbook

Landmark is the release-intelligence layer for factory repos. It owns release
truth: semantic version decisions, technical changelogs, release-note synthesis
status, and machine-readable evidence. A repo integrates Landmark by adding the
same small manifest and workflow shape, while keeping its existing local gate as
the release precondition.

## Required Files

1. Add `.landmark.yml` at the repository root:

```yaml
product:
  name: <Product>
  description: <One-line release context>
audience: developer
voice: Evidence-first, concrete, and release-operator friendly.
changelog:
  source: auto
release:
  profile: full
model:
  policy: balanced
budget:
  max_input_tokens: 12000
  max_output_tokens: 1200
  max_usd: 0.25
```

2. Add `.github/workflows/landmark-release.yml`:

```yaml
name: release intelligence

on:
  workflow_run:
    workflows:
      - <canonical gate workflow name>
    types:
      - completed
    branches:
      - master
  workflow_dispatch:

permissions:
  contents: write
  issues: write
  pull-requests: write

concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: false

jobs:
  landmark:
    if: github.event_name == 'workflow_dispatch' || github.event.workflow_run.conclusion == 'success'
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - name: Checkout repository history
        uses: actions/checkout@v4
        with:
          ref: ${{ github.event.workflow_run.head_sha || github.sha }}
          fetch-depth: 0
          persist-credentials: false

      - name: Run Landmark
        uses: misty-step/landmark@v1
        with:
          github-token: ${{ secrets.GH_RELEASE_TOKEN }}
          llm-api-key: ${{ secrets.OPENROUTER_API_KEY }}
          node-version: "24"
          synthesis: "true"
          synthesis-required: "false"
```

For repos whose protected branch is `main`, replace `master` with `main`. If a
repo already has a release workflow, attach Landmark after the existing release
creator in `synthesis-only` mode instead of adding a second full release job.

## Verification

- The repo's canonical gate must stay the release precondition. For
  `workflow_run` triggers, checkout must pin to
  `github.event.workflow_run.head_sha` so Landmark runs against the exact commit
  that passed the gate, not a newer default-branch commit.
- `GH_RELEASE_TOKEN` must have repository write access.
- `OPENROUTER_API_KEY` enables synthesized public notes; missing or stale keys
  must not block release unless the repo deliberately sets
  `synthesis-required: "true"`.
- Run the repo gate locally before opening the integration PR.

## Factory Coverage

This branch wires Threshold. The same playbook applies to powder, exocortex,
bitterblossom, crucible, cerberus, canary, and harness-kit on their own
branches, preserving each repo's canonical gate workflow name and default branch.
