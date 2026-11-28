---
status: DONE — QEMU bil-softcpu + rev4 chamber workload on main
next: optional independent SoftISA microarch; more chamber workloads
depends: []
---

# Checkpoint — bilkrnl — 2026-11-28

**Status:** Real QEMU `-device bil-softcpu` + second chamber workload (`rev4`) green.

## Done (with evidence)

- **QEMU softcpu:** `softcpu/qemu/` C SoftISA + SysBus device; `scripts/build-qemu-bil-softcpu.sh`; `SOFTCPU=1 ./scripts/run-qemu.sh`
  - Guest probe: MAGIC @ `0x0c000000` → `blast: … via softcpu`
  - Digests match 1A goldens for policy + rev4
- **Workload `rev4`:** sealed `BIL!` → Out `!LIB`; golden `33140a88…`; IR + fixtures + e2e
- Verify: `make -C softcpu/qemu test-isa`; `./scripts/verify.sh --only softcpu-qemu`

## Durable decisions

- Stock boot stays interpreter; softcpu is opt-in (`SOFTCPU=1`).
- Platform-bus base `0x0c000000` is the guest fast-path; DTB walk is backup.
- QEMU C SoftISA is a port of `bilkrnl-core::isa` (parity via `test-isa`); not yet a separate speculative-free microarch.

## Resume

```bash
./scripts/build-qemu-bil-softcpu.sh   # once
SOFTCPU=1 ./scripts/run-qemu.sh
./scripts/verify.sh --only softcpu-qemu
```
