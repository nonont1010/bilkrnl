#!/usr/bin/env bash
# Post-build verification for bilkrnl (layered tiers).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ONLY="${1:-}"
QUICK=0
if [[ "${1:-}" == "--quick" ]]; then
  QUICK=1
  ONLY=""
elif [[ "${1:-}" == "--only" ]]; then
  ONLY="${2:-}"
fi

pass() { echo "PASS: $*"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

build_core() {
  cargo test -p bilkrnl-tests --no-run -q
}

build_kernel() {
  cargo build -p bilkrnl --target aarch64-unknown-uefi --release -q
}

run_unit() {
  cargo test -p bil1asm -q
  cargo test -p bilkrnl-tests --test test_cap_poset --test test_budget \
    --test test_outbuf --test test_linear_as --test test_isa_faults \
    --test test_single_enclave --test test_chamber --test test_digest_oracle \
    --test test_lowered_goldens -q
  # 1C host MMIO soft-CPU seam (always; no QEMU fork required).
  cargo test -p softcpu -q
  # C SoftISA oracle used by the QEMU device (no forked QEMU required).
  run_softcpu_isa
}

run_security() {
  cargo test -p bilkrnl-tests --test test_image --test test_no_host_escape -q
}

run_integration() {
  cargo test -p bilkrnl-tests --test test_dispose --test test_zeroization -q
}

run_system_host() {
  cargo test -p bilkrnl-tests --test test_e2e_harness -q
  "$ROOT/tests/system/test_ratify_gate.sh" >/dev/null
  pass "ratify-gate fixtures (TC-16)"
}

run_ratify_gate_qemu() {
  SERIAL_LOG="$ROOT/target/qemu-serial.log"
  "$ROOT/scripts/ratify-gate.sh" --log "$SERIAL_LOG" || fail "ratify-gate digest mismatch"
  pass "ratify-gate qemu consumer (TC-16)"
}

run_boot_qemu() {
  build_kernel
  SERIAL_LOG="$ROOT/target/qemu-serial.log"
  TIMEOUT_SEC=45 "$ROOT/scripts/run-qemu.sh"
  grep -q "bilkrnl: ready" "$SERIAL_LOG" || fail "boot marker missing"
  pass "boot marker"
}

run_e2e_qemu() {
  SERIAL_LOG="$ROOT/target/qemu-serial.log"
  grep -q "chamber: RATIFIED" "$SERIAL_LOG" || fail "chamber ratified marker missing"
  pass "qemu chamber e2e (TC-01)"
}

run_serial_parity() {
  SERIAL_LOG="$ROOT/target/qemu-serial.log"
  # Policy auto-blast (`chamber: RATIFIED`) must match CLI blast; ignore other workloads
  # (e.g. `chamber:rev4: RATIFIED`, `chamber:rot4: RATIFIED`).
  mapfile -t chamber < <(grep -oE 'chamber: RATIFIED [0-9a-f]{64}' "$SERIAL_LOG" | awk '{print $3}')
  mapfile -t cli < <(grep -oE 'cli: RATIFIED [0-9a-f]{64}' "$SERIAL_LOG" | awk '{print $3}')
  [[ ${#chamber[@]} -ge 1 && ${#cli[@]} -ge 1 ]] || fail "need chamber: + cli: RATIFIED digests"
  [[ "${chamber[0]}" == "${cli[0]}" ]] || fail "cli/harness digest mismatch: ${chamber[0]} vs ${cli[0]}"
  pass "serial parity (TC-02)"
}

run_perf() {
  # TC-15: policy-gate PASS is 15 SoftISA ops; chamber max_cycles=128 is the hard ratchet.
  cargo test -p bilkrnl-tests --test test_e2e_harness -q
  "$ROOT/tests/perf/test_cycle_ratchet.sh" >/dev/null
  pass "perf cycle ratchet (TC-15)"
}

run_torture() {
  build_kernel
  # TC-12: kill QEMU mid-run — must not leave a false-green ratified-only log without ready...halt.
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
  # If log somehow contains RATIFIED without halt, that would be bad; after kill we require
  # verify treats this run as non-success for "full session". Marker: no bilkrnl: halt.
  if grep -q "bilkrnl: halt" "$LOG"; then
    fail "torture kill still saw clean halt (kill too late?)"
  fi
  if grep -q "chamber: RATIFIED" "$LOG" && ! grep -q "bilkrnl: ready" "$LOG"; then
    fail "false ratified without ready"
  fi
  # Torture stage itself succeeds when kill prevented clean halt (non-green full boot).
  pass "qemu kill torture (TC-12)"
}

check_1c_roadmap() {
  [[ -f "$ROOT/docs/roadmap/1c-softcpu.md" ]] || fail "missing 1C roadmap"
  [[ -f "$ROOT/softcpu/qemu/bil-softcpu.c" ]] || fail "missing QEMU bil-softcpu device source"
  pass "1C roadmap + QEMU device sources present"
}

run_softcpu_isa() {
  make -C "$ROOT/softcpu/qemu" test-isa >/dev/null
  pass "softcpu C SoftISA parity (test-isa)"
}

run_softcpu_qemu() {
  local fork="$ROOT/target/qemu-bil-softcpu/bin/qemu-system-aarch64"
  if [[ ! -x "$fork" ]]; then
    echo "SKIP: forked QEMU missing (./scripts/build-qemu-bil-softcpu.sh)"
    return 0
  fi
  build_kernel
  SERIAL_LOG="$ROOT/target/qemu-serial.log"
  TIMEOUT_SEC=45 SOFTCPU=1 "$ROOT/scripts/run-qemu.sh"
  grep -q "softcpu: probed" "$SERIAL_LOG" || fail "softcpu probe missing"
  grep -q "blast: policy via softcpu" "$SERIAL_LOG" || fail "policy not via softcpu"
  grep -q "blast: rev4 via softcpu" "$SERIAL_LOG" || fail "rev4 not via softcpu"
  grep -q "blast: rot4 via softcpu" "$SERIAL_LOG" || fail "rot4 not via softcpu"
  grep -q "chamber: RATIFIED 14ad56f22141c89095b78d1eb28cd846b3ad2c331a5c693f24a9a68711b317ea" "$SERIAL_LOG" \
    || fail "policy digest mismatch under softcpu"
  grep -q "chamber:rev4: RATIFIED 33140a88274a2f81b7d3b245b7506a020f6180debd32cc89b575d2343a646f09" "$SERIAL_LOG" \
    || fail "rev4 digest mismatch under softcpu"
  grep -q "chamber:rot4: RATIFIED 24784198062f6a05cc8d2047440574f644cd1eb961784864857294aecf5b7161" "$SERIAL_LOG" \
    || fail "rot4 digest mismatch under softcpu"
  pass "softcpu QEMU e2e (policy + rev4 + rot4)"
}

refresh_docs() {
  # Post-build docs refresh: regenerate neon-grid SVG/PNG kit (showcasing-graphics-in-docs).
  if [[ ! -f "$ROOT/docs/index.html" ]]; then
    fail "missing docs/index.html"
  fi
  python3 "$ROOT/scripts/render-graphics.py" || fail "render-graphics.py"
  [[ -f "$ROOT/docs/assets/logo.svg" ]] || fail "missing docs/assets/logo.svg"
  [[ -f "$ROOT/docs/assets/logo.png" ]] || fail "missing docs/assets/logo.png"
  [[ -f "$ROOT/docs/assets/examples/architecture.svg" ]] || fail "missing architecture.svg"
  [[ -f "$ROOT/docs/.nojekyll" ]] || fail "missing docs/.nojekyll"
  pass "docs graphics refresh"
}

# Optional host-side formal specs via bsommers/bil (Bounded Invariant Logic).
# Never linked into the UEFI kernel — CLI only. Skip cleanly if bil is absent.
run_bil_specs() {
  local specs=(
    "$ROOT/specs/bil/enclave_chamber.bil"
    "$ROOT/specs/bil/softisa_step.bil"
    "$ROOT/specs/bil/chamber_ratify.bil"
  )
  local bil_bin=""
  if command -v bil >/dev/null 2>&1; then
    bil_bin="$(command -v bil)"
  elif [[ -n "${BIL_BIN:-}" && -x "${BIL_BIN}" ]]; then
    bil_bin="${BIL_BIN}"
  elif [[ -x /tmp/bil-clone/target/release/bil ]]; then
    bil_bin=/tmp/bil-clone/target/release/bil
  elif [[ -x "$ROOT/../../lang/bil/target/release/bil" ]]; then
    bil_bin="$ROOT/../../lang/bil/target/release/bil"
  fi
  if [[ -z "$bil_bin" ]]; then
    echo "SKIP: bil CLI not found (install https://github.com/bsommers/bil or set BIL_BIN)"
    return 0
  fi
  local spec
  for spec in "${specs[@]}"; do
    [[ -f "$spec" ]] || fail "missing $spec"
    "$bil_bin" check "$spec" || fail "bil check $(basename "$spec")"
    "$bil_bin" model-check "$spec" || fail "bil model-check $(basename "$spec")"
  done
  pass "bil specs (enclave_chamber + softisa_step + chamber_ratify)"
}

case "${ONLY}" in
  "")
    build_core
    run_unit
    pass "unit"
    run_security
    pass "security"
    run_integration
    pass "integration"
    run_system_host
    pass "system-host"
    if [[ "$QUICK" -eq 1 ]]; then
      build_kernel
      pass "kernel build (quick)"
      check_1c_roadmap
      run_bil_specs
      refresh_docs
      echo "verify.sh --quick: OK"
      exit 0
    fi
    run_boot_qemu
    run_e2e_qemu
    run_serial_parity
    run_softcpu_qemu
    run_ratify_gate_qemu
    run_perf
    run_torture
    check_1c_roadmap
    run_bil_specs
    refresh_docs
    echo "verify.sh: ALL GREEN"
    ;;
  boot)
    run_boot_qemu
    ;;
  unit-cap)
    cargo test -p bilkrnl-tests --test test_cap_poset --test test_budget -q
    ;;
  unit-isa)
    cargo test -p bilkrnl-tests --test test_linear_as --test test_isa_faults --test test_outbuf -q
    ;;
  integration-enclave)
    run_integration
    cargo test -p bilkrnl-tests --test test_single_enclave -q
    ;;
  chamber)
    cargo test -p bilkrnl-tests --test test_chamber --test test_e2e_harness -q
    ;;
  ratify-gate)
    "$ROOT/tests/system/test_ratify_gate.sh"
    if [[ -f "$ROOT/target/qemu-serial.log" ]]; then
      run_ratify_gate_qemu
    else
      echo "note: no qemu-serial.log yet; fixture checks only"
    fi
    ;;
  docs)
    refresh_docs
    ;;
  bil-specs)
    run_bil_specs
    ;;
  softcpu)
    cargo test -p softcpu -q
    pass "softcpu host MMIO parity"
    run_softcpu_isa
    ;;
  softcpu-qemu)
    run_softcpu_qemu
    ;;
  *)
    echo "usage: verify.sh [--quick] | [--only boot|unit-cap|unit-isa|integration-enclave|chamber|ratify-gate|docs|bil-specs|softcpu|softcpu-qemu]" >&2
    exit 2
    ;;
esac
