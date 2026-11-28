# QEMU `-device bil-softcpu` Registration

> **Status:** Device sources + SoftISA C port live in [`softcpu/qemu/`](./qemu/). Host MMIO sim (`softcpu` Rust crate) remains the in-process seam. Build the forked binary with [`scripts/build-qemu-bil-softcpu.sh`](../scripts/build-qemu-bil-softcpu.sh). Stock `scripts/verify.sh` / default `run-qemu.sh` do **not** require the fork. Guest DTB probe is separate work.

## What landed

| Path | Role |
| :--- | :--- |
| `softcpu/qemu/bil_isa.{h,c}` | SoftISA matching `bilkrnl-core::isa::Vm` (opcodes, budgets, caps, digests) |
| `softcpu/qemu/sha256.{h,c}` | Public-domain SHA-256 for `code`/`out`/`artifact` digests |
| `softcpu/qemu/bil-softcpu.c` | `SysBusDevice` MMIO map from [`DEVICE.md`](./DEVICE.md) |
| `softcpu/qemu/test_bil_isa.c` | Host harness (no QEMU link) — PARITY fixtures |
| `softcpu/qemu/meson.build.snippet` / `Kconfig.snippet` / `patches/` | Upstream QEMU integration |
| `scripts/build-qemu-bil-softcpu.sh` | Clone ≈v10.2.0 → integrate → `aarch64-softmmu` → install under `target/qemu-bil-softcpu/` |

## Non-negotiable contracts

A fork may add a device. It may not retune these against `bilkrnl-core`:

- `BytecodeImage` encode/decode (`BIL1`, 32-byte header)
- Opcode map in `bilkrnl-core::isa::Op`, including `0xE0..=0xEF` host-escape traps
- `EnclaveResult` fields and `artifact_digest` framing
- Blast chamber order: dispose (`CMD_RESET_ZEROIZE`) then `chamber_ratify`
- Capability poset / ratchet (`Cap`, weaken-only)

The 1A interpreter remains the oracle. Digests must match [`PARITY.md`](./PARITY.md). SoftISA harness: `make -C softcpu/qemu test-isa`.

## Type registration

User-facing qdev type `bil-softcpu`, parent `TYPE_SYS_BUS_DEVICE`:

- `user_creatable = true`
- prop `max-pages` (uint16, default 256)
- MMIO: little-endian, 64 KiB aperture (`BIL_SOFTCPU_MMIO_SIZE`)
- `machine_class_allow_dynamic_sysbus_dev(mc, TYPE_BIL_SOFTCPU)` on `virt`
- FDT binding in `hw/core/sysbus-fdt.c` emits `compatible = "bil,softcpu"`, `reg` = bus base + `0x10000`

Platform bus places the region in the virt platform-bus window; guest reads `reg` from DTB (guest MMIO driver still out of scope).

## Run opt-in

```bash
./scripts/build-qemu-bil-softcpu.sh   # once
SOFTCPU=1 ./scripts/run-qemu.sh       # appends -device bil-softcpu,max-pages=256
# or: QEMU_BIN=target/qemu-bil-softcpu/bin/qemu-system-aarch64 ./scripts/run-qemu.sh
```

Default `./scripts/run-qemu.sh` stays stock `qemu-system-aarch64` with no soft-CPU device. `scripts/verify.sh` does not pass `-device bil-softcpu`.

## Design-only header

[`include/bil_softcpu.h`](./include/bil_softcpu.h) stays uncompiled into bilkrnl (`#error` under `BIL_SOFTCPU_BUILD` / `BILKRNL_BUILD`). The QEMU tree uses `softcpu/qemu/bil_softcpu_regs.h` + `include/hw/misc/bil_softcpu.h` (type macros only).

## Interpreter fallback

Absence of the device (stock QEMU / no DTB node / bad MAGIC) keeps the 1A `isa::Vm` path. Digests are the PARITY.md goldens. A failed `COMMIT_LOAD` after a successful MAGIC probe is an error, not a silent replay onto `Vm`.
