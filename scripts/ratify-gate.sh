#!/usr/bin/env bash
# Host consumer of bilkrnl chamber ratification.
#
# Exit 0 only when serial output contains
#   chamber: RATIFIED <digest>
# and <digest> equals the sealed policy-gate PASS golden.
# This is the external "someone else cares about RATIFIED" check —
# it never trusts a live enclave run, only the disposed-then-ratified marker.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GOLDEN_DEFAULT="$ROOT/tests/system/golden/policy_pass.artifact_digest"
SERIAL_DEFAULT="$ROOT/target/qemu-serial.log"

usage() {
  cat <<'EOF' >&2
usage:
  ratify-gate.sh [--log FILE] [--golden FILE]   check an existing serial log
  ratify-gate.sh --qemu [--golden FILE]         build kernel, run QEMU, then check
  ratify-gate.sh --help

Exit codes: 0 match, 1 mismatch/missing/runtime, 2 usage.
EOF
  exit 2
}

LOG=""
GOLDEN="$GOLDEN_DEFAULT"
MODE="check"
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage ;;
    --log)
      [[ $# -ge 2 ]] || usage
      LOG="$2"
      shift 2
      ;;
    --golden)
      [[ $# -ge 2 ]] || usage
      GOLDEN="$2"
      shift 2
      ;;
    --qemu)
      MODE="qemu"
      shift
      ;;
    *)
      echo "error: unknown argument '$1'" >&2
      usage
      ;;
  esac
done

if [[ ! -f "$GOLDEN" ]]; then
  echo "error: golden missing: $GOLDEN" >&2
  exit 1
fi
expect="$(tr -d '[:space:]' <"$GOLDEN")"
if [[ ! "$expect" =~ ^[0-9a-f]{64}$ ]]; then
  echo "error: golden must be 64 lowercase hex chars" >&2
  exit 1
fi

if [[ "$MODE" == "qemu" ]]; then
  cargo build -p bilkrnl --target aarch64-unknown-uefi --release -q
  SERIAL_LOG="${SERIAL_LOG:-$SERIAL_DEFAULT}"
  TIMEOUT_SEC="${TIMEOUT_SEC:-45}" "$ROOT/scripts/run-qemu.sh"
  LOG="${LOG:-$SERIAL_LOG}"
fi

LOG="${LOG:-$SERIAL_DEFAULT}"
if [[ ! -f "$LOG" ]]; then
  echo "error: serial log missing: $LOG" >&2
  exit 1
fi

# Prefer the auto-blast chamber marker; fall back to any RATIFIED line.
mapfile -t digests < <(grep -oE 'chamber: RATIFIED [0-9a-f]{64}' "$LOG" | awk '{print $3}')
if [[ ${#digests[@]} -eq 0 ]]; then
  mapfile -t digests < <(grep -oE 'RATIFIED [0-9a-f]{64}' "$LOG" | awk '{print $2}')
fi
if [[ ${#digests[@]} -eq 0 ]]; then
  echo "error: no RATIFIED <digest> in $LOG" >&2
  exit 1
fi

got="${digests[0]}"
if [[ "$got" != "$expect" ]]; then
  echo "error: ratify gate failed: got $got want $expect" >&2
  exit 1
fi

# If multiple chamber markers exist, all must match the golden.
for d in "${digests[@]}"; do
  if [[ "$d" != "$expect" ]]; then
    echo "error: ratify gate failed: mixed digests (saw $d)" >&2
    exit 1
  fi
done

echo "ratify-gate: OK $got"
exit 0
