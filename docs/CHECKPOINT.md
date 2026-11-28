---
status: COMPLETE — QEMU bil-softcpu + rev4 on main @ b0e6f8b; safe to /compact
next: optional next workload or SoftISA-body independence; else pause
depends: []
---

# Checkpoint — bilkrnl — 2026-11-28

**Status:** COMPLETE for this session arc. Tree clean on `main` == `origin/main` @ `b0e6f8b`. Safe to `/compact` or `/clear`.

## Goal (user words)

Hardened UEFI aarch64 bilkrnl; SoftISA enclaves; blast chamber; workloads whose `RATIFIED <digest>` someone else consumes; soft-CPU path that is not just a host MMIO shim.

## Done (with evidence)

| Landed | Commit / proof |
|--------|----------------|
| bil MemOob/PcOob/HostEscape LTL witnesses | `c7f6eb8` |
| B3.2 `bil1asm lower` + rejects | `38a09a8` |
| B3.3 verify gate, B3.4 goldens, host `softcpu` MMIO | `9e3ca31` |
| **QEMU `-device bil-softcpu` + rev4 workload** | `b0e6f8b` |
| TC-16 policy golden | `chamber: RATIFIED 14ad56f2…` |
| rev4 golden | `chamber:rev4: RATIFIED 33140a88…` |
| GPL-3.0-or-later | `6549948` |

**SOFTCPU boot (verified this session):**
```text
softcpu: probed
blast: policy via softcpu
chamber: RATIFIED 14ad56f22141c89095b78d1eb28cd846b3ad2c331a5c693f24a9a68711b317ea
blast: rev4 via softcpu
chamber:rev4: RATIFIED 33140a88274a2f81b7d3b245b7506a020f6180debd32cc89b575d2343a646f09
cli: RATIFIED 14ad56f2…   # matches policy
```

**Gates:** `./scripts/verify.sh --quick` OK; `./scripts/verify.sh --only softcpu-qemu` OK; `make -C softcpu/qemu test-isa` OK.

## Durable decisions

- Stock QEMU path = interpreter; softcpu is **opt-in** (`SOFTCPU=1` / forked binary).
- Guest fast-path: MAGIC probe @ platform-bus `0x0c000000`; DTB walk is backup.
- QEMU C SoftISA (`softcpu/qemu/bil_isa.c`) is a **port** of `bilkrnl-core::isa` — digest-parity via `test-isa`, not a separate microarch yet.
- Host Rust `softcpu` crate remains the in-process MMIO seam (still wraps `Vm`).
- bil dump format still Rust Debug; `.bc.txt` preferred; `.bil` lower needs `bil verify` (B3.3) unless `--skip-verify`.
- Multi-account `gh`: `GH_TOKEN="$(gh auth token --user nonont1010)"` — do not `gh auth switch`.
- License: **GPL-3.0-or-later**.

## Bugs / gotchas this session (do not reintroduce)

- `chamber:rev4: RATIFIED` must **not** be matched by TC-16 / serial-parity greps for `chamber: RATIFIED` (space after colon).
- AAVMF may not expose a usable EFI DTB for platform-bus; MAGIC @ `0x0c000000` is required for guest softcpu.
- Full QEMU build needs `--disable-werror` on this host toolchain; binary lives under `target/qemu-bil-softcpu/` (gitignored).

## Working-tree state

- Branch: `main` == `origin/main`, clean.
- Forked QEMU (local only): `target/qemu-bil-softcpu/bin/qemu-system-aarch64` after `./scripts/build-qemu-bil-softcpu.sh`.
- bil CLI: `/home/bill/src/lang/bil/target/release/bil` (banner may say 0.7.1; sources 0.8.1).

## Next steps (exact; pick one)

1. **More chamber workload** — new sealed job + golden + dual blast marker (pattern: `rev4`).
2. **SoftISA independence** — stop sharing semantics with a silent `Vm` wrap in places that claim “device”; keep C SoftISA as sole 1C body, tighten host sim docs.
3. **Pause** — nothing required; V1+softcpu story is shippable for demos.

## Resume command

```bash
cd /home/bill/src/os/bilkrnl && git status -sb && ./scripts/verify.sh --quick
# softcpu demo (needs prior build-qemu-bil-softcpu.sh):
SOFTCPU=1 ./scripts/run-qemu.sh && rg 'softcpu:|chamber' target/qemu-serial.log
```
