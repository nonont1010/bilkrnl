# Milestone 1C — Custom soft-CPU

**Status:** Host MMIO **protocol** seam landed (`softcpu` crate, wraps `Vm`). **QEMU SoftISA C body + device** landed (`softcpu/qemu/bil_isa.c`, `make -C softcpu/qemu test-isa`). Forked `qemu-system-aarch64` is **optional** via `scripts/build-qemu-bil-softcpu.sh` / `SOFTCPU=1`. Guest MAGIC probe `@ 0x0c000000` selects the C device. RTL deferred. Stock path keeps the 1A interpreter.

**Notes:** [`softcpu/README.md`](../../softcpu/README.md), [`softcpu/DEVICE.md`](../../softcpu/DEVICE.md), [`softcpu/PARITY.md`](../../softcpu/PARITY.md), [`softcpu/qemu-device.md`](../../softcpu/qemu-device.md), [`docs/SOFTISA.md`](../SOFTISA.md), [`softcpu/include/bil_softcpu.h`](../../softcpu/include/bil_softcpu.h) (not compiled into kernel).

## Goal

Replace the in-kernel SoftISA interpreter body with a QEMU device model (and later FPGA/RTL) that executes the same `BytecodeImage` (`BIL1`) with identical `EnclaveResult` digests.

## Honesty split

| Component | Role |
|-----------|------|
| `softcpu` Rust crate | MMIO **protocol** simulator — still calls `isa::Vm` |
| `softcpu/qemu/bil_isa.c` | **Sole in-tree 1C SoftISA body** (digest-parity port) |
| Kernel without MAGIC | 1A `Vm` fallback |

## Non-negotiable contracts (must stay byte-identical)

- `BytecodeImage` encode/decode (`BIL1`, 32-byte header)
- Opcode map in `bilkrnl-core::isa::Op` (including `0xE0..=0xEF` host-escape traps)
- `EnclaveResult` fields and `artifact_digest` hashing
- Blast chamber: dispose → `chamber_ratify` (2C)
- Capability poset / ratchet (`Cap`, weaken-only)

## Acceptance

1. [x] **Host seam:** Fixture suite produces the same digests via MMIO (`parity_via_mmio_*`) as the 1A oracle (protocol wrap of `Vm`).
2. [x] **QEMU device sources:** `-device bil-softcpu` type + SoftISA C port + `virt` allow-list; build via `scripts/build-qemu-bil-softcpu.sh`. Opt-in run: `SOFTCPU=1 ./scripts/run-qemu.sh`.
3. [x] Host aarch64 speculative execution remains irrelevant to enclave semantics.
4. [x] `cargo test -p softcpu` wired into `scripts/verify.sh`; SoftISA harness `make -C softcpu/qemu test-isa`.
5. [x] Guest MAGIC probe selects C device over `Vm` when present (`softcpu: probed` / `blast: … via softcpu`).
6. [ ] FPGA / RTL backend (out of tree OK).

## Layout

```
softcpu/                 # host Rust MMIO protocol sim + specs + include/bil_softcpu.h
softcpu/qemu/            # C SoftISA body + bil-softcpu SysBusDevice + test-isa harness
scripts/build-qemu-bil-softcpu.sh
docs/roadmap/1c-softcpu.md
```

## Explicitly still deferred for stock verify

- No forked QEMU required to pass `scripts/verify.sh`
- Kernel continues to ship the software interpreter as fallback
- RTL not in-tree
