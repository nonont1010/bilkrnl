# Milestone 1C — Soft-CPU Stub (`softcpu/`)

> **Status:** Design notes only — **NOT IMPLEMENTED**. Explicitly out of V1 verify gates (`scripts/verify.sh`). No RTL and no QEMU fork live here; the 1A in-kernel software interpreter (`bilkrnl-core::isa::Vm`) remains authoritative.
>
> Sketches (also out of `scripts/verify.sh`): [`include/bil_softcpu.h`](./include/bil_softcpu.h) (C API, **not compiled**) and [`qemu-device.md`](./qemu-device.md) (how a future fork registers `-device bil-softcpu`). [`DEVICE.md`](./DEVICE.md) is the MMIO map. [`PARITY.md`](./PARITY.md) is the digest oracle.

## Purpose

This directory reserves the home for Milestone 1C: a custom QEMU device model (`bil-softcpu`, and optionally a future FPGA/RTL backend) that replaces the in-kernel SoftISA interpreter body while executing the exact same `BytecodeImage` (`BIL1`) and emitting byte-identical `EnclaveResult` and blast-chamber digests. Offloading enclave execution to a non-speculative device model keeps host `aarch64` speculative execution irrelevant to enclave semantics without changing `bilkrnl`'s enclave load/run/dispose/ratify APIs.

## Non-negotiable contracts (must stay byte-identical)

Copied from [`docs/roadmap/1c-softcpu.md`](../docs/roadmap/1c-softcpu.md) — any future 1C implementation must preserve these contracts verbatim against `bilkrnl-core`:

- `BytecodeImage` encode/decode (`BIL1`, 32-byte header: 4B magic `BIL1`, `entry: u32`, `code_len: u32`, `initial_caps: u8` + 3B reserved, `max_cycles: u64`, `max_out_bytes: u32`, `max_mem_pages: u16` + 2B pad, followed by `code` up to 64 KiB)
- Opcode map in `bilkrnl-core::isa::Op` (`0x00` `Halt` through `0x0A` `Weaken`, including `0xE0..=0xEF` host-escape traps raising `FaultKind::HostEscapeAttempt(op)`)
- `EnclaveResult` fields (`exit`, `out`, `cycles`, `mem_high_water`, `code_digest`, `out_digest`, `caps_at_exit`) and `artifact_digest` SHA-256 framing (`exit_tag || cycles_le64 || out_len_le32 || out || code_digest || out_digest`)
- Blast chamber: dispose → `chamber_ratify` (2C) — enclave linear RAM and output buffers are zeroized on dispose before host ratification inspects `EnclaveResult`
- Capability poset / ratchet (`Cap`, weaken-only: `None (0) < Compute (1) < IoOutBounded (2)`)

## Suggested QEMU `-device bil-softcpu` shape

Registration, `virt` platform-bus glue, the ESP/kernel `BIL1` load path, and the interpreter fallback are in [`qemu-device.md`](./qemu-device.md). When 1C is wired into `scripts/run-qemu.sh` (future, opt-in, forked QEMU only), the board gains a sysbus device without altering UEFI boot or enclave call sites:

```text
qemu-system-aarch64 \
  -machine virt -cpu cortex-a72 -m 256M -nographic \
  -device bil-softcpu,mmio-base=0x0a001000,max-pages=256 \
  ...
```

- Exposed to `bilkrnl.efi` as a fixed MMIO window (or virtio-mmio / DTB-discovered sysbus region) — see [`DEVICE.md`](./DEVICE.md) for the proposed register map.
- The in-kernel 1A `Vm` interpreter stays the fallback for every boot, including a future tree that contains a device model. `EnclaveSlot` selects the device only after `MAGIC` reads `"BIL1"`. A missing window or a failed probe keeps `enclave_load` / `blast` on `Vm`. A `COMMIT_LOAD` rejection after the device was selected returns `LoadError`; it is not replayed on `Vm`. Callers still pass a `BytecodeImage`. They do not pass an MMIO base. Observable digests on the fallback path are the 1A results; the device, when present, must match them.

## Digest-parity acceptance vs 1A interpreter

Concrete programs, cycle counts, and expected digest hex live in [`PARITY.md`](./PARITY.md) (halt, budget exceed, host-escape `0xE0`, out fill / overflow, mem OOB). That file is a fixture plan only. It does not add `tests/` sources, and `scripts/verify.sh` does not run it.

Before 1C can graduate from stub to active backend:

1. The cases in [`PARITY.md`](./PARITY.md) — and the existing `tests/` programs they map to (valid halt, budget exhaustion, capability ratchet violations, OOB faults, and `0xE0..=0xEF` host-escape traps) — must produce identical `code_digest`, `out_digest`, and `artifact_digest` on both the 1A software interpreter and the 1C `bil-softcpu` device. The interpreter result is the oracle.
2. Serial output markers (`chamber: RATIFIED <64-hex>`) under QEMU with `-device bil-softcpu` must match the 1A QEMU run byte-for-byte on `artifact_digest`.
3. Post-dispose zeroization probes must confirm device-side linear memory and output buffers read back all-zero prior to `chamber_ratify`.

## Explicitly out of V1 verify gates

- **Not gated by `scripts/verify.sh`:** V1 verification checks only that the 1C roadmap note exists; it requires **no QEMU fork**, **no custom C/Rust device build**, and **no RTL/HDL toolchain**.
- **No TCB expansion in V1:** Markdown notes plus `include/bil_softcpu.h`, which is a sketch and is never compiled or linked into `bilkrnl-core`, `bilkrnl`, or `scripts/verify.sh`. No Meson or CMake target in this repository builds it.
