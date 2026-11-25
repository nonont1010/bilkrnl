#!/usr/bin/env bash
# TC-01 system: QEMU boot → chamber RATIFIED
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
"$ROOT/scripts/verify.sh" --only boot
grep -q "chamber: RATIFIED" "$ROOT/target/qemu-serial.log"
echo "TC-01 OK"
