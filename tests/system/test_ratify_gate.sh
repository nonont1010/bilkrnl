#!/usr/bin/env bash
# TC-16: external host consumer of chamber: RATIFIED <golden>.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
GATE="$ROOT/scripts/ratify-gate.sh"
GOLDEN="$ROOT/tests/system/golden/policy_pass.artifact_digest"
FIX="$ROOT/target/ratify-gate-fixtures"
mkdir -p "$FIX"
EXPECT=$(tr -d '[:space:]' < "$GOLDEN")

pass_log="$FIX/pass.log"
fail_log="$FIX/wrong.log"
empty_log="$FIX/empty.log"
wrong=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

{
  printf 'bilkrnl: ready\r\n'
  printf 'chamber: RATIFIED %s\r\n' "$EXPECT"
  printf 'bilkrnl: halt\r\n'
} >"$pass_log"

{
  printf 'bilkrnl: ready\r\n'
  printf 'chamber: RATIFIED %s\r\n' "$wrong"
} >"$fail_log"

printf 'bilkrnl: ready\r\n' >"$empty_log"

"$GATE" --log "$pass_log" --golden "$GOLDEN" >/dev/null

set +e
"$GATE" --log "$fail_log" --golden "$GOLDEN" >/dev/null 2>"$FIX/wrong.err"
rc=$?
set -e
if [[ "$rc" -ne 1 ]]; then
  echo "expected exit 1 for wrong digest, got $rc" >&2
  exit 1
fi
grep -q "ratify gate failed" "$FIX/wrong.err"

set +e
"$GATE" --log "$empty_log" --golden "$GOLDEN" >/dev/null 2>"$FIX/empty.err"
rc=$?
set -e
if [[ "$rc" -ne 1 ]]; then
  echo "expected exit 1 for missing RATIFIED, got $rc" >&2
  exit 1
fi
grep -q "no RATIFIED" "$FIX/empty.err"

echo "TC-16 OK"
