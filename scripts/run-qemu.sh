#!/usr/bin/env bash
# Boot bilkrnl.efi under QEMU aarch64 virt + AAVMF UEFI.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_MODE="${BUILD_MODE:-release}"
TARGET="aarch64-unknown-uefi"
EFI_SRC="$ROOT/target/$TARGET/$BUILD_MODE/bilkrnl.efi"
ESP="$ROOT/target/esp"
FIRMWARE_CODE="${FIRMWARE_CODE:-/usr/share/AAVMF/AAVMF_CODE.fd}"
FIRMWARE_VARS_TEMPLATE="${FIRMWARE_VARS_TEMPLATE:-/usr/share/AAVMF/AAVMF_VARS.fd}"
VARS_COPY="$ROOT/target/AAVMF_VARS.fd"
TIMEOUT_SEC="${TIMEOUT_SEC:-30}"
SERIAL_LOG="${SERIAL_LOG:-$ROOT/target/qemu-serial.log}"

if [[ ! -f "$EFI_SRC" ]]; then
  echo "missing EFI: $EFI_SRC (build kernel first)" >&2
  exit 1
fi

if [[ ! -f "$FIRMWARE_CODE" ]]; then
  echo "missing UEFI firmware: $FIRMWARE_CODE" >&2
  exit 1
fi

rm -rf "$ESP"
mkdir -p "$ESP/EFI/BOOT"
cp "$EFI_SRC" "$ESP/EFI/BOOT/BOOTAA64.EFI"
cp "$FIRMWARE_VARS_TEMPLATE" "$VARS_COPY"

ARGS=(
  qemu-system-aarch64
  -machine virt
  -cpu cortex-a72
  -m 256M
  -nographic
  -drive if=pflash,format=raw,readonly=on,file="$FIRMWARE_CODE"
  -drive if=pflash,format=raw,file="$VARS_COPY"
  -drive format=raw,file="fat:rw:$ESP"
)

if [[ "${INTERACTIVE:-0}" == "1" ]]; then
  exec "${ARGS[@]}"
fi

# Non-interactive: capture serial, enforce timeout, always tear down.
set +e
timeout --signal=KILL "$TIMEOUT_SEC" "${ARGS[@]}" >"$SERIAL_LOG" 2>&1
rc=$?
set -e

if [[ $rc -eq 124 || $rc -eq 137 ]]; then
  echo "qemu timed out after ${TIMEOUT_SEC}s" >&2
  exit 1
fi

# UEFI shutdown often yields non-zero; success is defined by serial markers.
exit 0
