#!/usr/bin/env bash
set -euo pipefail

daedalus-score findings.json tests/expected.json
