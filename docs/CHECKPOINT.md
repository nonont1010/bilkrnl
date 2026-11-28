---
status: COMPLETE — SoftISA docs + rot4 + graphics suite landed on main
next: optional more workload / RTL / pause — `cd /home/bill/src/os/bilkrnl && ./scripts/verify.sh --quick`
depends: []
---

# Checkpoint — bilkrnl — 2026-11-28

**Status:** COMPLETE
**Branch:** `main` (clean; synced with `origin/main`; SoftISA/rot4 feature at `e9dabf2`)

## Goal (user words)

Instruction set datasheet + programmer guide; then a new chamber workload (rev4 pattern); then SoftISA-body independence (C as sole 1C body; host sim docs honest); then full docs/graphics refresh, commit, and push.

## Done (with evidence)

| Landed | Proof |
|--------|-------|
| SoftISA datasheet + guide | `docs/SOFTISA.md`, `docs/SOFTISA-GUIDE.md` (+ hub links) |
| **rot4** sealed left-rotate workload | `BIL1` → Out `IL1B`; marker `chamber:rot4:` |
| rot4 golden | `24784198062f6a05cc8d2047440574f644cd1eb961784864857294aecf5b7161` |
| SoftISA independence | C `bil_isa.c` = 1C body; Rust `softcpu` = MMIO protocol wrap of `Vm` |
| Docs + neon-grid graphics suite | `docs/diagrams/`, `docs/assets/examples/softisa.{svg,png}`, `docs/index.html` |
| Commit + push | `e9dabf2` on `origin/main` — *docs: SoftISA datasheet/guide, rot4 workload, and graphic suite* |

**Prior (still on tree):** QEMU bil-softcpu, rev4, policy TC-16, GPL-3.0-or-later (`b0e6f8b`, `70cada2`).

## Durable decisions

- Stock QEMU path = interpreter; softcpu is **opt-in** (`SOFTCPU=1` / forked binary).
- Guest fast-path: MAGIC probe @ platform-bus `0x0c000000`; DTB walk is backup.
- QEMU C SoftISA (`softcpu/qemu/bil_isa.c`) is the **sole in-tree 1C SoftISA body** — digest-parity via `test-isa`.
- Host Rust `softcpu` crate is **not** a SoftISA body; it wraps `Vm` for MMIO protocol tests.
- Dual-marker greps: `chamber: RATIFIED` (space) must not match `chamber:rev4:` / `chamber:rot4:`.
- License: **GPL-3.0-or-later**.

## Failed / blocked

None this arc.

## Next steps (exact; pick one)

1. **More chamber workload** — another sealed job + golden + marker.
2. **RTL / FPGA** — out-of-tree OK.
3. **Pause** — SoftISA docs + three chamber workloads + softcpu story are demo-ready.

## Resume command

```bash
cd /home/bill/src/os/bilkrnl && git status -sb && ./scripts/verify.sh --quick
# softcpu demo (needs prior build-qemu-bil-softcpu.sh):
SOFTCPU=1 ./scripts/run-qemu.sh && rg 'softcpu:|chamber' target/qemu-serial.log
```
