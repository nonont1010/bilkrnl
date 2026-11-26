---
status: DONE — Wave2+Wave3 locally; commit+push when asked
next: Commit+push Wave2/3; then B3.1 assembler or 1C QEMU device
depends: []
---

# Checkpoint — bilkrnl — 2026-11-25

**Status:** Wave2+Wave3 complete locally (uncommitted). Review: **approve-with-nits**.

## Goal (user words)

Hardened UEFI aarch64 bilkrnl with soft-ISA enclaves, blast chamber (dispose+ratify), KVM/QEMU test. Docs with neon visuals. Incorporate bsommers/bil (host specs). Re-render figures with diagrams-as-code. Record lumagrid for future charts.

## Done (with evidence)

- V1 kernel + brand on `origin/main`; Wave0+Wave1 `29f226f`.
- **Wave2:** chamber `OutTooLarge`/`CapCeilingViolated` tests; `tests/parity/test_digest_oracle` (7 goldens); softcpu C header + `qemu-device.md`; CHECKPOINT sync.
- **Wave3:** B1 refine `softisa_step.bil` (439 states VERIFIED); B3 design `docs/roadmap/b3-lowering.md` + `tools/bil1asm/`; fresh-eyes review approve-with-nits.
- Orchestrator: wired `test_digest_oracle` into `verify.sh` unit; B2 Rust checkbox; softisa comment nits.

## User decisions (durable)

- Execution **1A**, blast chamber **2C**; later **1C** with QEMU soft-CPU.
- Remote `github.com/nonont1010/bilkrnl`; commit/push when asked.
- diagrams-as-code for architecture figures; lumagrid OK for future web charts.
- Logo: TRON neon techno-corn + geometric **`bilkrnl`** wordmark.

## Working-tree state

- Dirty: Wave2/3 tests, softcpu scaffold, B1 refine, B3 docs, verify wiring.

## Next steps

1. User asks → commit + push Wave2/3.
2. Optional: bil reachability witnesses for MemOob/PcOob/HostEscape.
3. B3.1 host assembler (profile IR → BIL1) or 1C QEMU device from `softcpu/`.
