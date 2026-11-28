# Milestone 1C — Custom soft-CPU

**Status:** Host MMIO seam landed (`softcpu` crate). **QEMU SoftISA C device sources + host harness landed** (`softcpu/qemu/`, `make -C softcpu/qemu test-isa`). Forked `qemu-system-aarch64` is **optional** via `scripts/build-qemu-bil-softcpu.sh` / `SOFTCPU=1`. RTL deferred. Interpreter path (1A) is still the UEFI kernel body (guest DTB probe separate).

**Notes:** [`softcpu/README.md`](../../softcpu/README.md), [`softcpu/DEVICE.md`](../../softcpu/DEVICE.md), [`softcpu/PARITY.md`](../../softcpu/PARITY.md), [`softcpu/qemu-device.md`](../../softcpu/qemu-device.md), [`softcpu/include/bil_softcpu.h`](../../softcpu/include/bil_softcpu.h) (not compiled into kernel).

## Goal

Replace the in-kernel SoftISA interpreter body with a QEMU device model (and later FPGA/RTL) that executes the same `BytecodeImage` (`BIL1`) with identical `EnclaveResult` digests.

## Non-negotiable contracts (must stay byte-identical)

- `BytecodeImage` encode/decode (`BIL1`, 32-byte header)
- Opcode map in `bilkrnl-core::isa::Op` (including `0xE0..=0xEF` host-escape traps)
- `EnclaveResult` fields and `artifact_digest` hashing
- Blast chamber: dispose → `chamber_ratify` (2C)
- Capability poset / ratchet (`Cap`, weaken-only)

## Acceptance

1. [x] **Host seam:** Fixture suite produces the same digests via MMIO (`parity_via_mmio_*`) as the 1A oracle.
2. [x] **QEMU device sources:** `-device bil-softcpu` type + SoftISA C port + `virt` allow-list; build via `scripts/build-qemu-bil-softcpu.sh`. Opt-in run: `SOFTCPU=1 ./scripts/run-qemu.sh`. Guest probe still deferred.
3. [x] Host aarch64 speculative execution remains irrelevant to enclave semantics.
4. [x] `cargo test -p softcpu` wired into `scripts/verify.sh`; SoftISA harness `make -C softcpu/qemu test-isa` (optional, not required for stock verify).
5. [ ] FPGA / RTL backend (out of tree OK).
6. [ ] Guest DTB / MMIO driver selecting device backend over `Vm`.

## Layout

```
softcpu/                 # host Rust MMIO sim + specs + include/bil_softcpu.h
softcpu/qemu/            # C SoftISA + bil-softcpu SysBusDevice + test-isa harness
scripts/build-qemu-bil-softcpu.sh
docs/roadmap/1c-softcpu.md
```

## Explicitly still deferred for stock verify

- No forked QEMU required to pass `scripts/verify.sh`
- Kernel continues to ship the software interpreter as fallback / EFI path
- RTL not in-tree
