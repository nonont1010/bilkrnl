#!/usr/bin/env bash
# TC-15: demo fixture must Halt within chamber max_cycles (64); soft bound 16.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
cargo test -p bilkrnl-tests --test test_e2e_harness -q
echo "TC-15 OK"
