#!/usr/bin/env bash
# TC-02: auto blast and cli blast digests match
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
LOG="$ROOT/target/qemu-serial.log"
[[ -f "$LOG" ]] || "$ROOT/scripts/verify.sh" --only boot
mapfile -t digests < <(grep -oE 'RATIFIED [0-9a-f]{64}' "$LOG" | awk '{print $2}')
[[ ${#digests[@]} -ge 2 ]]
[[ "${digests[0]}" == "${digests[1]}" ]]
echo "TC-02 OK"
