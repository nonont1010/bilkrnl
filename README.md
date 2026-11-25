# bilkrnl

Hardened research kernel: UEFI aarch64 boot (QEMU/KVM), software ISA enclaves
(6502-style direct execution), capability poset/ratchet, and a blast chamber that
**disposes** enclave state then **ratifies** results with no trust that the run was safe.

## Quick start

```bash
scripts/verify.sh --quick   # unit + kernel build
scripts/verify.sh           # full tiers including QEMU
INTERACTIVE=1 scripts/run-qemu.sh
```

## Layout

- `bilkrnl-core` — caps, ISA, enclave, chamber (`no_std`)
- `kernel` — UEFI entry, serial CLI
- `tests/` — exhaustive matrix (unit/integration/security/system/perf/torture)
- `docs/roadmap/1c-softcpu.md` — deferred custom soft-CPU (1C)

## Locked decisions

- **1A now:** in-kernel soft ISA interpreter
- **2C:** dispose + result ratification
- **1C later:** QEMU soft-CPU / RTL with identical `BIL1` contracts
