---
status: DONE — TC-16 host ratify-gate on main; verify ALL GREEN
next: optional bil witnesses / tighter Rust↔bil refinement; then B3.2 only if wanted; 1C last
depends: []
---

# Checkpoint — bilkrnl — 2026-11-25

**Status:** COMPLETE for host ratify-gate (TC-16). Safe to `/compact`.

## Goal (user words)

Hardened UEFI aarch64 bilkrnl; SoftISA enclaves; blast chamber; a workload whose `RATIFIED <digest>` someone else consumes.

## Done (with evidence)

- Killer job **A** (sealed policy gate) + B3.1 `bil1asm` @ `49bd2e1` on `origin/main`.
- **TC-16 host ratify-gate**:
  - `scripts/ratify-gate.sh` — exit 0 only if `chamber: RATIFIED` matches golden
  - Golden: `tests/system/golden/policy_pass.artifact_digest` = `14ad56f22141c89095b78d1eb28cd846b3ad2c331a5c693f24a9a68711b317ea`
  - Fixtures: `tests/system/test_ratify_gate.sh` (pass / wrong / missing)
  - Wired into `verify.sh` (fixtures in system-host; live QEMU check after serial parity)
- `./scripts/verify.sh` → **ALL GREEN** (includes `ratify-gate: OK 14ad56f2…`).

## Durable decisions

- 1A SoftISA + 2C chamber; **1C deferred** until digests are boring.
- bil host-side only; never in UEFI TCB.
- Policy gate weakens to **Compute** (not None) — Halt needs Compute.
- Skip logo polish / 1C for now.
- Next: optional bil witnesses → B3.2 only if wanted → 1C last.

## Working-tree state

- Branch: `main` (TC-16 + docs). No secrets. Docs venv: `.venv-docs`. bil: `/tmp/bil-clone/target/release/bil`.

## Next steps (exact)

1. Optional: bil reachability witnesses / Rust↔bil refinement.
2. Optional: B3.2 bil→BIL1 lowering.
3. **Not yet:** 1C soft-CPU; logo polish.

## Resume command

```bash
cd /home/bill/src/os/bilkrnl && git status -sb && ./scripts/verify.sh --quick
```
