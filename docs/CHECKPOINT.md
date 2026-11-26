---
status: DONE — brand shipped; commit+push when asked
next: Commit + push TRON corn brand (user typically asks)
depends: []
---

# Checkpoint — bilkrnl — 2026-11-25

**Status:** Brand complete locally; working tree dirty until commit.

## Goal (user words)

Hardened UEFI aarch64 bilkrnl with soft-ISA enclaves, blast chamber (dispose+ratify), KVM/QEMU test. Docs with neon visuals. Incorporate bsommers/bil (host specs). Re-render figures with diagrams-as-code. Record lumagrid for future charts. **Latest:** custom retro-futuristic TRON-style neon techno-corn-kernel + husk/shell logo with cool retro font spelling `bilkrnl`.

## Done (with evidence)

- Greenfield kernel + `bilkrnl-core` SoftISA / caps / enclave / chamber; `scripts/verify.sh` ALL GREEN (incl. QEMU e2e).
- Repo: https://github.com/nonont1010/bilkrnl · Pages: https://nonont1010.github.io/bilkrnl/
- Locked decisions: **1A** soft ISA now; **2C** dispose+ratify; **1C** soft-CPU deferred (`docs/roadmap/1c-softcpu.md`).
- bil (Bounded Invariant Logic): `specs/bil/enclave_chamber.bil` model-checks clean; never link into UEFI TCB.
- Docs suite + diagrams-as-code figures; **lumagrid** noted in `docs/TOOLING.md`.
- **TRON corn brand:** `docs/diagrams/brand_tron_corn.py` → `docs/assets/logo.{svg,png}`, `icon.{svg,png}`, `assets/logo.*`, category icons. Wired in `scripts/render-graphics.py`. `./scripts/verify.sh --only docs` PASS.

## User decisions (durable)

- Execution **1A**, blast chamber **2C**; later **1C** with QEMU soft-CPU.
- Remote `github.com/nonont1010/bilkrnl`; commit/push when asked.
- diagrams-as-code for architecture figures; lumagrid OK for future web charts.
- Logo: **retro-futuristic TRON-style neon techno-corn-kernel and shell**, geometric retro **`bilkrnl`** wordmark (no webfonts).

## Working-tree state

- Brand generator + re-rendered assets + TOOLING/CHECKPOINT updates uncommitted.
- Docs venv: `.venv-docs` (gitignored).

## Next steps

1. User asks → commit + push brand assets.
