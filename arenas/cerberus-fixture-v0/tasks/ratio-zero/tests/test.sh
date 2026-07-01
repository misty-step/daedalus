#!/usr/bin/env bash
set -euo pipefail

threshold-score findings.json tests/expected.json
