#!/usr/bin/env sh
# Phase 0 verifier: scores findings.json against the answer key, printing the
# scorer JSON (reward field). Harbor-compatible entrypoint for Phase 1.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
WORKDIR=${1:-$PWD}
if command -v threshold-score >/dev/null 2>&1; then
  threshold-score "$WORKDIR/findings.json" "$HERE/expected.json"
else
  threshold score "$WORKDIR/findings.json" "$HERE/expected.json"
fi
