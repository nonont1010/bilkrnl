---
status: DONE — B3.3 + B3.4 + 1C host MMIO seam
next: optional QEMU `-device bil-softcpu` fork / independent SoftISA body; more chamber workloads
depends: []
---

# Checkpoint — bilkrnl — 2026-11-28

**Status:** B3.3, B3.4, and 1C host seam green. QEMU fork / RTL deferred. Safe to `/compact`.

## Done (with evidence)

- **B3.3:** `bil1asm lower foo.bil` → `bil verify --allow-bounded` first; `--skip-verify`; 31 bil1asm tests.
- **B3.4:** `test_lowered_goldens` (ok / out_hello / add_imm) → 1A digests; verify unit tier.
- **1C host seam:** `softcpu` MMIO sim (DEVICE.md); body still `Vm`; `parity_via_mmio_*` (8); `--only softcpu`.
- Prior: witnesses `c7f6eb8`, B3.2 `38a09a8`, TC-16, GPL.

## Durable decisions

- 1C *host* ≠ QEMU device. Digests via MMIO; SoftISA body remains `bilkrnl-core::isa::Vm` until fork/RTL.
- `.bc.txt` skips verify; `.bil` needs bil (or `--skip-verify`).
- bil dump still Rust Debug.

## Resume command

```bash
cd /home/bill/src/os/bilkrnl && cargo test -p bil1asm && cargo test -p softcpu && ./scripts/verify.sh --quick
```
