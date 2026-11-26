# Milestone 1C — Custom soft-CPU (deferred)

**Status:** Not in V1 verify gates. Interpreter path (1A) is authoritative until this lands.

**Spike notes (design only, not implemented):** [`softcpu/README.md`](../../softcpu/README.md), [`softcpu/DEVICE.md`](../../softcpu/DEVICE.md), [`softcpu/PARITY.md`](../../softcpu/PARITY.md), [`softcpu/qemu-device.md`](../../softcpu/qemu-device.md), and the non-compiled sketch [`softcpu/include/bil_softcpu.h`](../../softcpu/include/bil_softcpu.h). `scripts/verify.sh` still only checks that this roadmap file exists.

## Goal

Replace the in-kernel SoftISA interpreter body with a QEMU device model (and later FPGA/RTL) that executes the same `BytecodeImage` (`BIL1`) with identical `EnclaveResult` digests.

## Non-negotiable contracts (must stay byte-identical)

- `BytecodeImage` encode/decode (`BIL1`, 32-byte header)
- Opcode map in `bilkrnl-core::isa::Op` (including `0xE0..=0xEF` host-escape traps)
- `EnclaveResult` fields and `artifact_digest` hashing
- Blast chamber: dispose → `chamber_ratify` (2C)
- Capability poset / ratchet (`Cap`, weaken-only)

## Acceptance

1. Fixture suite under `tests/` produces the same `code_digest`, `out_digest`, and `artifact_digest` on 1A interpreter and 1C soft-CPU.
2. QEMU invocation gains a soft-CPU device (e.g. `-device bil-softcpu`) without changing enclave load APIs.
3. Host aarch64 speculative execution remains irrelevant to enclave semantics.

## Suggested layout (future)

```
softcpu/           # notes + include/bil_softcpu.h (not compiled) — no device binary
rtl/               # optional HDL — out of tree OK
docs/roadmap/1c-softcpu.md  # this file
```

## Explicitly out of V1

- No RTL in-tree
- No QEMU fork submodule required to pass `scripts/verify.sh`
- Kernel continues to ship the software interpreter as fallback
