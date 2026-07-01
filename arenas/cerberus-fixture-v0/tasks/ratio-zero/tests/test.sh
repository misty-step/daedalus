#!/usr/bin/env bash
set -euo pipefail

if command -v threshold-score >/dev/null 2>&1; then
  threshold-score findings.json tests/expected.json
else
  threshold score findings.json tests/expected.json
fi
