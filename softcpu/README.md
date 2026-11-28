# Milestone 1C — Soft-CPU (`softcpu/`)

> **Status:** **Host MMIO simulator** (`cargo test -p softcpu`) **and** **QEMU SoftISA device sources** (`softcpu/qemu/`, `make -C softcpu/qemu test-isa`) are landed. Forked binary is **optional** (`scripts/build-qemu-bil-softcpu.sh` → `SOFTCPU=1 ./scripts/run-qemu.sh`). FPGA/RTL remain deferred. The 1A in-kernel interpreter stays the UEFI kernel body until guest DTB probe lands.

Specs: [`DEVICE.md`](./DEVICE.md), [`PARITY.md`](./PARITY.md), [`qemu-device.md`](./qemu-device.md), design-only [`include/bil_softcpu.h`](./include/bil_softcpu.h) (keep `#error`).

## Layers

| Layer | Location | SoftISA body |
| :--- | :--- | :--- |
| Host Rust MMIO seam | `softcpu` crate | Still calls `bilkrnl-core::isa::Vm` behind registers |
| C SoftISA + QEMU device | `softcpu/qemu/` | Own port of `Vm` (opcodes / budgets / caps / digests) |
| Stock verify / run | `scripts/verify.sh`, default `run-qemu.sh` | No fork required |

## Host Rust seam

Protocol: `MAGIC` → `BEGIN_LOAD` → FIFO → `COMMIT_LOAD` → `RUN` → result window → `RESET_ZEROIZE`.

```bash
cargo test -p softcpu
```

## C SoftISA harness (PARITY.md)

```bash
make -C softcpu/qemu test-isa
```

Fixtures: `halt`, `budget_exceed`, `host_escape_e0`, `out_fill_ok` / `out_fill_overflow`, `mem_oob`. Digests must match PARITY.md hex.

## QEMU `-device bil-softcpu` (optional)

```bash
./scripts/build-qemu-bil-softcpu.sh
SOFTCPU=1 ./scripts/run-qemu.sh
```

Installs `target/qemu-bil-softcpu/bin/qemu-system-aarch64`. Guest MMIO driver / DTB probe is separate work; without it the kernel still uses in-tree `Vm`.

## Non-negotiable contracts

- `BytecodeImage` encode/decode (`BIL1`)
- Opcode map including `0xE0..=0xEF` host-escape traps
- `EnclaveResult` / `artifact_digest` framing
- Dispose → `chamber_ratify`
- Cap weaken-only poset

## Deferred

- Guest EFI DTB probe / MMIO driver
- FPGA / RTL
- Compiling `include/bil_softcpu.h` into bilkrnl
