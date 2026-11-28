# Milestone 1C — Soft-CPU (`softcpu/`)

> **Status:** **Host MMIO protocol sim** (`cargo test -p softcpu`, wraps `Vm`) **and** **QEMU SoftISA device** (`softcpu/qemu/bil_isa.c`, `make -C softcpu/qemu test-isa`) are landed. Forked binary is **optional** (`scripts/build-qemu-bil-softcpu.sh` → `SOFTCPU=1 ./scripts/run-qemu.sh`). Guest MAGIC probe `@ 0x0c000000` selects the C device when present. FPGA/RTL remain deferred. Stock QEMU / missing MAGIC keeps the 1A in-kernel interpreter.

Specs: [`DEVICE.md`](./DEVICE.md), [`PARITY.md`](./PARITY.md), [`qemu-device.md`](./qemu-device.md), design-only [`include/bil_softcpu.h`](./include/bil_softcpu.h) (keep `#error`). Instruction set: [`docs/SOFTISA.md`](../docs/SOFTISA.md).

## Layers (read carefully)

| Layer | Location | SoftISA body? |
| :--- | :--- | :--- |
| Host Rust MMIO seam | `softcpu` crate | **No** — wraps `bilkrnl-core::isa::Vm` (protocol / register tests) |
| C SoftISA + QEMU device | `softcpu/qemu/` | **Yes** — independent port; digest parity via `test-isa` |
| Stock verify / run | `scripts/verify.sh`, default `run-qemu.sh` | No fork required; kernel uses 1A `Vm` |

Do not call the Rust `softcpu` crate “the SoftISA device” without the protocol qualifier. The sole 1C SoftISA body in-tree is the C port.

## Host Rust seam (protocol)

Protocol: `MAGIC` → `BEGIN_LOAD` → FIFO → `COMMIT_LOAD` → `RUN` → result window → `RESET_ZEROIZE`.

```bash
cargo test -p softcpu
```

## C SoftISA harness (body parity)

```bash
make -C softcpu/qemu test-isa
```

Fixtures: `halt`, `budget_exceed`, `host_escape_e0`, `out_fill_ok` / `out_fill_overflow`, `mem_oob`. Digests must match PARITY.md hex.

## QEMU `-device bil-softcpu` (optional)

```bash
./scripts/build-qemu-bil-softcpu.sh
SOFTCPU=1 ./scripts/run-qemu.sh
```

Installs `target/qemu-bil-softcpu/bin/qemu-system-aarch64`. Guest probes MAGIC at platform-bus `0x0c000000` (DTB walk is backup). Expect `softcpu: probed` and `blast: … via softcpu`.

## Non-negotiable contracts

- `BytecodeImage` encode/decode (`BIL1`)
- Opcode map including `0xE0..=0xEF` host-escape traps
- `EnclaveResult` / `artifact_digest` framing
- Dispose → `chamber_ratify`
- Cap weaken-only poset

## Deferred

- FPGA / RTL
- Compiling `include/bil_softcpu.h` into bilkrnl
