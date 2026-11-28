# Milestone 1C — Custom soft-CPU

**Status:** Host MMIO seam landed (`softcpu` crate). QEMU device fork and RTL remain deferred. Interpreter path (1A) is still the UEFI kernel body.

**Notes:** [`softcpu/README.md`](../../softcpu/README.md), [`softcpu/DEVICE.md`](../../softcpu/DEVICE.md), [`softcpu/PARITY.md`](../../softcpu/PARITY.md), [`softcpu/qemu-device.md`](../../softcpu/qemu-device.md), and the non-compiled sketch [`softcpu/include/bil_softcpu.h`](../../softcpu/include/bil_softcpu.h).

## Goal

Replace the in-kernel SoftISA interpreter body with a QEMU device model (and later FPGA/RTL) that executes the same `BytecodeImage` (`BIL1`) with identical `EnclaveResult` digests. Host-side first step: an in-process MMIO simulator that speaks the same register protocol.

## Non-negotiable contracts (must stay byte-identical)

- `BytecodeImage` encode/decode (`BIL1`, 32-byte header)
- Opcode map in `bilkrnl-core::isa::Op` (including `0xE0..=0xEF` host-escape traps)
- `EnclaveResult` fields and `artifact_digest` hashing
- Blast chamber: dispose → `chamber_ratify` (2C)
- Capability poset / ratchet (`Cap`, weaken-only)

## Acceptance

1. [x] **Host seam:** Fixture suite produces the same `code_digest`, `out_digest`, and `artifact_digest` via MMIO (`parity_via_mmio_*` in `softcpu`) as the 1A oracle (`test_digest_oracle`).
2. [ ] QEMU invocation gains a soft-CPU device (e.g. `-device bil-softcpu`) without changing enclave load APIs.
3. [x] Host aarch64 speculative execution remains irrelevant to enclave semantics (host sim / future device; kernel still uses SoftISA `Vm`).
4. [x] `cargo test -p softcpu` wired into `scripts/verify.sh` (unit tier + `--only softcpu`); no skip when present.
5. [ ] FPGA / RTL backend (out of tree OK).

## Layout

```
softcpu/           # host Rust MMIO sim + notes + include/bil_softcpu.h (not compiled)
rtl/               # optional HDL — out of tree OK
docs/roadmap/1c-softcpu.md  # this file
```

## Explicitly still deferred

- No RTL in-tree
- No QEMU fork submodule required to pass `scripts/verify.sh`
- Kernel continues to ship the software interpreter as fallback / EFI path
