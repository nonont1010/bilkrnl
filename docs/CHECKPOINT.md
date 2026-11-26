---
status: DONE — Wave0+Wave1 (B0/B1/B2 specs + 1C softcpu notes); uncommitted
next: Commit+push when asked; then B1 refine / B2 Rust Reject tests / B3 or 1C implement
depends: []
---

# Checkpoint — bilkrnl — 2026-11-25

**Status:** V1 kernel verified; brand shipped; Wave0+Wave1 complete — bil B0/B1/B2 host specs model-check; `softcpu/` design spike; `run_bil_specs` gates all three `.bil` files.

## Goal (user words)

Hardened UEFI aarch64 bilkrnl with soft-ISA enclaves, blast chamber (dispose+ratify), KVM/QEMU test. Docs with neon visuals. Incorporate bsommers/bil (host specs). Re-render figures with diagrams-as-code. Record lumagrid for future charts. **Latest:** custom retro-futuristic TRON-style neon techno-corn-kernel + husk/shell logo with cool retro font spelling `bilkrnl`.

## Done (with evidence)

- Greenfield kernel + `bilkrnl-core` SoftISA / caps / enclave / chamber; `scripts/verify.sh` ALL GREEN (incl. QEMU e2e).
- Repo: https://github.com/nonont1010/bilkrnl · Pages: https://nonont1010.github.io/bilkrnl/
- Locked decisions: **1A** soft ISA now; **2C** dispose+ratify; **1C** soft-CPU deferred (`docs/roadmap/1c-softcpu.md`).
- bil (Bounded Invariant Logic): `enclave_chamber.bil` + `softisa_step.bil` (B1) + `chamber_ratify.bil` (B2) model-check; never link into UEFI TCB; `run_bil_specs` / `--only bil-specs`.
- Docs suite + diagrams-as-code figures; **lumagrid** noted in `docs/TOOLING.md`.
- **TRON corn brand:** `docs/diagrams/brand_tron_corn.py` → logo/icon assets. Wired in `scripts/render-graphics.py`.
- **1C spike notes:** `softcpu/{README,DEVICE,PARITY}.md` (design only; interpreter remains fallback).

## User decisions (durable)

- Execution **1A**, blast chamber **2C**; later **1C** with QEMU soft-CPU.
- Remote `github.com/nonont1010/bilkrnl`; commit/push when asked.
- diagrams-as-code for architecture figures; lumagrid OK for future web charts.
- Logo: **retro-futuristic TRON-style neon techno-corn-kernel and shell**, geometric retro **`bilkrnl`** wordmark (no webfonts).

## Working-tree state

- Uncommitted: `specs/bil/{softisa_step,chamber_ratify}.bil`, `softcpu/`, roadmap/CHECKPOINT/verify.sh updates. Docs venv: `.venv-docs` (gitignored).

## Next steps

1. User asks → commit + push Wave0/Wave1 artifacts.
2. B1 refine: pc/acc/mem abstraction (Jmp/Jz PcOob) in `softisa_step.bil`.
3. B2 follow-up: Rust unit tests for `OutTooLarge` / `CapCeilingViolated`.
4. B3 lowering (stretch) or 1C QEMU device implementation from `softcpu/` notes.