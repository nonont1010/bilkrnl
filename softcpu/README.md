# Milestone 1C — Soft-CPU host seam (`softcpu/`)

> **Status:** **Host MMIO simulator implemented** (`cargo test -p softcpu`). QEMU `-device bil-softcpu` fork and FPGA/RTL remain **deferred**. The 1A in-kernel interpreter (`bilkrnl-core::isa::Vm`) stays authoritative for the UEFI kernel path; this crate does not change `bilkrnl.efi` or require a forked `qemu-system-aarch64`.
>
> Specs: [`DEVICE.md`](./DEVICE.md) (MMIO map), [`PARITY.md`](./PARITY.md) (digest goldens), [`include/bil_softcpu.h`](./include/bil_softcpu.h) (C sketch, **not compiled** — keep the `#error` guard), [`qemu-device.md`](./qemu-device.md) (future fork notes).

## Purpose

Host-side Rust MMIO device simulator matching the register map in `DEVICE.md` / `bil_softcpu.h`. Guest-facing load APIs (`BytecodeImage`, `enclave_load` / `blast`) are unchanged. A future QEMU device can reuse the same protocol; V1 verify only needs this in-process seam.

## Honest SoftISA body (same `Vm` until a fork)

After `COMMIT_LOAD`, execution calls **`bilkrnl-core::isa::Vm`** for `STEP` / `RUN`. Digests therefore match the 1A oracle by construction. This crate owns the **MMIO protocol** (registers, load → run → result window → `RESET_ZEROIZE`), not a second opcode decoder. A future SoftISA fork would replace the `Vm` body behind the same registers.

## Protocol

```text
probe MAGIC (0x314C4942)
BEGIN_LOAD → stream BIL1 via IMG_FIFO → COMMIT_LOAD
RUN → read EXIT / CYCLES / OUT / DIGESTS → rebuild EnclaveResult
RESET_ZEROIZE → ZERO_VERIFIED == 1
```

Host helpers: `SoftCpu`, `blast_via_mmio`, `enclave_result_from_mmio`, `artifact_digest_from_mmio`.

## Parity tests

`cargo test -p softcpu` runs `parity_via_mmio_*` for the classic PARITY.md fixtures (`halt`, `budget_exceed`, `host_escape_e0`, `out_fill_ok`, `out_fill_overflow`, `mem_oob`). Wired into `scripts/verify.sh` unit tier (also `--only softcpu`). The 1A oracle `tests/parity/test_digest_oracle.rs` is untouched.

## Non-negotiable contracts (byte-identical with `bilkrnl-core`)

- `BytecodeImage` encode/decode (`BIL1`, 32-byte header)
- Opcode map in `bilkrnl-core::isa::Op` (including `0xE0..=0xEF` host-escape traps)
- `EnclaveResult` fields and `artifact_digest` framing
- Blast chamber: dispose → `chamber_ratify` (2C)
- Capability poset / ratchet (`Cap`, weaken-only)

## Deferred (not in this crate)

- QEMU fork registering `-device bil-softcpu` / DTB discovery
- Guest EFI driver probing MMIO instead of in-kernel `Vm`
- FPGA / RTL backends
- Compiling `include/bil_softcpu.h` (design-only; `#error` under build macros)
