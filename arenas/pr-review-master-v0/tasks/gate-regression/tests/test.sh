#!/usr/bin/env sh
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
WORKDIR=${1:-$PWD}
python3 "$HERE/../../../../../runner/score.py" "$WORKDIR/findings.json" "$HERE/expected.json"
