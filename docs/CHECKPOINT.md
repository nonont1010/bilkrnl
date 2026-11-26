---
status: DONE — Wave2/3 + brand redesign shipped (3d4b72d on origin/main)
next: Fresh session — pick B3.1 assembler OR 1C QEMU device OR logo proportion polish
depends: []
---

# Checkpoint — bilkrnl — 2026-11-25

**Status:** COMPLETE for this session arc. Tree clean on `origin/main` @ `3d4b72d`. Safe to `/compact` or `/clear`.

## Goal (user words)

Hardened UEFI aarch64 bilkrnl: SoftISA enclaves, capability ratchet, blast chamber (dispose→ratify), QEMU/KVM. Host-side bil specs. TRON corn brand. Parallel wave execution for bil B1–B3 / 1C notes.

## Done (with evidence)

- **V1 kernel** green; brand v1 earlier; Wave0+Wave1 @ `29f226f`.
- **Wave2+Wave3 @ `3d4b72d`:**
  - Chamber tests: `OutTooLarge`, `CapCeilingViolated` (`tests/unit/test_chamber.rs`)
  - 1A digest oracle: `tests/parity/test_digest_oracle.rs` (7 goldens) wired into `verify.sh` unit
  - B1 refine: `specs/bil/softisa_step.bil` — 439 states VERIFIED
  - B3 design: `docs/roadmap/b3-lowering.md`, `tools/bil1asm/`
  - 1C scaffold: `softcpu/include/bil_softcpu.h`, `softcpu/qemu-device.md`
  - Review: approve-with-nits (reachability witnesses still optional)
- **Brand redesign @ same commit:** stockier cob (~1:1.2), dropped `1A`/`2C`/`1C…` chips; tagline *Isolate. Step. Dispose. Ratify. Trust nothing.*; architecture rail + CAP↓; icon = chamber hex only. Generator: `docs/diagrams/brand_tron_corn.py`.

## User decisions (durable)

- Execution **1A**, blast chamber **2C**; **1C** deferred soft-CPU.
- bil host-side only — **never** link into UEFI TCB.
- Remote `github.com/nonont1010/bilkrnl`; push via `gh auth switch -u nonont1010`.
- diagrams-as-code for architecture figures; lumagrid OK for future charts.
- Logo: TRON techno-corn + husk; geometric wordmark; architecture/purpose visual — **not** roadmap milestone chips.
- Parallel waves OK with model routing (composer cheap / sonnet TDD / opus formal / grok 1C explore).

## Working-tree state

- Clean: `main` == `origin/main` @ `3d4b72d`.
- Docs venv: `.venv-docs` (gitignored). bil CLI: `/tmp/bil-clone/target/release/bil`.

## Next steps (pick one)

1. **Logo polish** (if still too tall): tweak `rx`/`ry` in `corn_emblem`, re-run brand render, commit.
2. **B3.1:** host profile-IR → `BIL1` assembler under `tools/bil1asm/` (golden `ok.bil1.txt`).
3. **1C implement:** QEMU `-device bil-softcpu` from `softcpu/` notes + parity oracles.
4. Optional: bil reachability witnesses for MemOob/PcOob/HostEscape in `softisa_step.bil`.

## Resume command

```bash
cd /home/bill/src/os/bilkrnl && git status -sb && ./scripts/verify.sh --quick
```

Then read this checkpoint and choose next step 1–4.
