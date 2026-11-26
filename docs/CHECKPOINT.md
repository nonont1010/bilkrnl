---
status: DONE — B3.1 + sealed policy gate on QEMU; verify ALL GREEN
next: Optional commit; then bil witnesses or leave digests boring before 1C
depends: []
---

# Checkpoint — bilkrnl — 2026-11-25

**Status:** COMPLETE for killer job A + B3.1. `./scripts/verify.sh` ALL GREEN.

## Killer enclave job

**Sealed policy gate.** Candidate + allowlist sealed in `BIL1`. Emits `PASS\n`/`FAIL\n`, weakens IoOutBounded→**Compute** (Halt needs Compute), chamber ratifies ExactBytes + caps ceiling Compute. Consumer: CI/agent on `RATIFIED <golden>`.

PASS `artifact_digest`: `14ad56f22141c89095b78d1eb28cd846b3ad2c331a5c693f24a9a68711b317ea`

## Shipped

- `tools/bil1asm` (assemble/dump) + `ok` / `policy_pass` / `policy_fail` IR
- Core fixtures + `demo_policy_*_spec`; kernel/CLI demo on PASS path
- Parity goldens; verify unit tier runs `bil1asm` + digest oracle

## Next

1. Commit when you ask.
2. Optional bil witnesses / tighter refinement.
3. 1C only after digests stay boring.

## Resume

```bash
cd /home/bill/src/os/bilkrnl && git status -sb && ./scripts/verify.sh --quick
```
