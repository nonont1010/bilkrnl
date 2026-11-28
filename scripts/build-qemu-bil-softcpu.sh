#!/usr/bin/env bash
# Build a forked qemu-system-aarch64 with -device bil-softcpu.
# Idempotent. Sources live in softcpu/qemu/; QEMU tree is gitignored under target/.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${QEMU_SRC:-$ROOT/target/qemu-bil-src}"
PREFIX="${QEMU_PREFIX:-$ROOT/target/qemu-bil-softcpu}"
DEV_SRC="$ROOT/softcpu/qemu"
JOBS="${JOBS:-$(nproc 2>/dev/null || echo 4)}"
TAG="${QEMU_TAG:-v10.2.0}"
FALLBACK_TAGS=(v10.2.0 v10.1.0 v10.0.0)

log() { printf '[build-qemu-bil-softcpu] %s\n' "$*"; }

clone_qemu() {
  if [[ -d "$SRC/.git" ]]; then
    log "QEMU source present: $SRC ($(git -C "$SRC" describe --tags --always 2>/dev/null || echo unknown))"
    return 0
  fi
  mkdir -p "$(dirname "$SRC")"
  local t
  for t in "$TAG" "${FALLBACK_TAGS[@]}"; do
    log "Cloning QEMU $t into $SRC ..."
    if git clone --depth 1 --branch "$t" https://gitlab.com/qemu-project/qemu.git "$SRC"; then
      log "Cloned $t"
      return 0
    fi
    rm -rf "$SRC"
  done
  log "ERROR: failed to clone QEMU"
  exit 1
}

integrate_device() {
  log "Integrating bil-softcpu into $SRC"
  mkdir -p "$SRC/include/hw/misc"
  # Device + SoftISA sources into hw/misc/
  for f in bil-softcpu.c bil_isa.c bil_isa.h sha256.c sha256.h bil_softcpu_regs.h; do
    cp -f "$DEV_SRC/$f" "$SRC/hw/misc/$f"
  done
  cp -f "$DEV_SRC/include/hw/misc/bil_softcpu.h" "$SRC/include/hw/misc/bil_softcpu.h"

  # meson.build
  if ! grep -q "CONFIG_BIL_SOFTCPU" "$SRC/hw/misc/meson.build"; then
    cat "$DEV_SRC/meson.build.snippet" >>"$SRC/hw/misc/meson.build"
    log "Appended meson.build snippet"
  else
    log "meson.build already has BIL_SOFTCPU"
  fi

  # Kconfig
  if ! grep -q "config BIL_SOFTCPU" "$SRC/hw/misc/Kconfig"; then
    cat "$DEV_SRC/Kconfig.snippet" >>"$SRC/hw/misc/Kconfig"
    log "Appended Kconfig snippet"
  else
    log "Kconfig already has BIL_SOFTCPU"
  fi

  # virt allow-list (idempotent edit)
  if ! grep -q "TYPE_BIL_SOFTCPU" "$SRC/hw/arm/virt.c"; then
    if ! grep -q 'hw/misc/bil_softcpu.h' "$SRC/hw/arm/virt.c"; then
      # Insert include after ramfb.h
      sed -i '/#include "hw\/display\/ramfb.h"/a #include "hw/misc/bil_softcpu.h"' \
        "$SRC/hw/arm/virt.c"
    fi
    # Insert allow-list after SMMUV3 line
    sed -i '/machine_class_allow_dynamic_sysbus_dev(mc, TYPE_ARM_SMMUV3);/a\    machine_class_allow_dynamic_sysbus_dev(mc, TYPE_BIL_SOFTCPU);' \
      "$SRC/hw/arm/virt.c"
    log "Patched hw/arm/virt.c allow-list"
  else
    log "virt.c already allows bil-softcpu"
  fi

  # sysbus-fdt binding (required for dynamic instantiate on virt)
  python3 "$DEV_SRC/patches/apply-sysbus-fdt.py" "$SRC/hw/core/sysbus-fdt.c"
}

configure_and_build() {
  cd "$SRC"
  # Drop a stale empty/non-meson build dir so configure can recreate it.
  if [[ -d build && ! -f build/build.ninja ]]; then
    log "Removing stale build/ (no build.ninja)"
    rm -rf build
  fi
  if [[ ! -f build/build.ninja ]]; then
    log "Configuring (aarch64-softmmu, docs/tools off) ..."
    # --disable-werror: newer host GCC flags warnings in bundled dtc/libfdt.
    ./configure \
      --target-list=aarch64-softmmu \
      --prefix="$PREFIX" \
      --disable-docs \
      --disable-user \
      --disable-tools \
      --disable-guest-agent \
      --disable-sdl \
      --disable-gtk \
      --disable-vnc \
      --disable-werror \
      --enable-tcg \
      || {
        log "configure failed; retrying with fewer disable flags"
        ./configure \
          --target-list=aarch64-softmmu \
          --prefix="$PREFIX" \
          --disable-docs \
          --disable-werror \
          --enable-tcg
      }
  else
    log "Already configured (build/build.ninja exists)"
  fi

  log "Building with ninja -j$JOBS ..."
  # Reconfigure once if a prior tree was built with -Werror and dtc fails.
  if ! ninja -C build -j"$JOBS"; then
    log "ninja failed; wiping build and reconfiguring with --disable-werror"
    rm -rf build
    ./configure \
      --target-list=aarch64-softmmu \
      --prefix="$PREFIX" \
      --disable-docs \
      --disable-werror \
      --enable-tcg
    ninja -C build -j"$JOBS"
  fi
  log "Installing to $PREFIX ..."
  ninja -C build install
}

print_next() {
  local bin="$PREFIX/bin/qemu-system-aarch64"
  cat <<EOF

=== bil-softcpu QEMU ready ===
Binary: $bin
SoftISA harness (no QEMU):  make -C softcpu/qemu test-isa
Run with device (opt-in):   SOFTCPU=1 ./scripts/run-qemu.sh
  (or QEMU_BIN=$bin ./scripts/run-qemu.sh)
Stock path unchanged:       ./scripts/run-qemu.sh
Verify stock still green:   ./scripts/verify.sh --quick

EOF
  if [[ -x "$bin" ]]; then
    "$bin" -device help 2>/dev/null | grep -i bil-softcpu || \
      log "NOTE: -device help did not list bil-softcpu (check CONFIG_BIL_SOFTCPU)"
  fi
}

main() {
  clone_qemu
  integrate_device
  # Marker for any parallel waiter
  touch "$DEV_SRC/.ready-for-qemu-integrate"
  configure_and_build
  print_next
}

main "$@"
