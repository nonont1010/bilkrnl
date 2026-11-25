#!/usr/bin/env bash
# TC-12 wrapper — delegates to verify torture path via full verify segment.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# Build kernel then run kill scenario inline (same as verify.sh run_torture).
cargo build -p bilkrnl --target aarch64-unknown-uefi --release -q
ESP="$ROOT/target/esp-torture"
rm -rf "$ESP"
mkdir -p "$ESP/EFI/BOOT"
cp "$ROOT/target/aarch64-unknown-uefi/release/bilkrnl.efi" "$ESP/EFI/BOOT/BOOTAA64.EFI"
VARS="$ROOT/target/AAVMF_VARS_torture.fd"
cp /usr/share/AAVMF/AAVMF_VARS.fd "$VARS"
LOG="$ROOT/target/qemu-torture.log"
set +e
qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 256M -nographic \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/AAVMF/AAVMF_CODE.fd \
  -drive if=pflash,format=raw,file="$VARS" \
  -drive format=raw,file="fat:rw:$ESP" \
  >"$LOG" 2>&1 &
qpid=$!
sleep 0.3
kill -KILL "$qpid" 2>/dev/null
wait "$qpid" 2>/dev/null
set -e
if grep -q "bilkrnl: halt" "$LOG"; then
  echo "kill too late" >&2
  exit 1
fi
echo "TC-12 OK"
