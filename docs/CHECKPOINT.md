---
status: DONE — bil MemOob/PcOob/HostEscape reachability witnesses + TC-13 witness sync
next: optional B3.2 bil→BIL1 lowering (needs stable bil dump); 1C last
depends: []
---

# Checkpoint — bilkrnl — 2026-11-27

**Status:** COMPLETE for bil reachability witnesses / Rust↔bil refinement. Safe to `/compact`.

## Goal (user words)

Hardened UEFI aarch64 bilkrnl; SoftISA enclaves; blast chamber; a workload whose `RATIFIED <digest>` someone else consumes.

## Done (with evidence)

- Killer job **A** (sealed policy gate) + B3.1 `bil1asm` @ `49bd2e1` on `origin/main`.
- **TC-16 host ratify-gate** + **GPL-3.0-or-later** on main.
- **Bil reachability witnesses** (Wave 3 review follow-up):
  - `MemOobReachWitness` / `PcOobReachWitness` / `HostEscapeReachWitness` in `specs/bil/softisa_step.bil`
  - LTL `eventually(fault == …)` **PROVED** under weak fairness (2 states each)
  - SoftIsaStep: operand-past-end PcOob + AddImm keep-class; still **439** states VERIFIED
  - Chamber TC-13 witnesses renamed to match Rust (`tc13_out_too_large_reject`, `tc13_cap_ceiling_reject`); verify 24/24
  - `verify.sh` finds sibling `../../lang/bil/target/release/bil`

## Durable decisions

- 1A SoftISA + 2C chamber; **1C deferred** until digests are boring.
- bil host-side only; never in UEFI TCB.
- SoftIsaStep is demonic — reachability of MemOob/PcOob/HostEscape needs separate witness systems (not `eventually` on SoftIsaStep itself).
- Skip logo polish / 1C for now.
- License: **GPL-3.0-or-later**.
- Multi-account `gh`: do **not** `gh auth switch`; use `GH_TOKEN="$(gh auth token --user nonont1010)"` for this repo.
- Next: optional B3.2 → 1C last.

## Working-tree state

- Branch: `main` (dirty: witnesses + docs + verify bil path). No secrets.
- Docs venv: `.venv-docs`. bil: `/home/bill/src/lang/bil/target/release/bil` (or `BIL_BIN` / `/tmp/bil-clone`).

## Next steps (exact)

1. Optional: B3.2 bil→BIL1 lowering (wants stable bil dump format).
2. **Not yet:** 1C soft-CPU; logo polish.

## Resume command

```bash
cd /home/bill/src/os/bilkrnl && git status -sb && ./scripts/verify.sh --only bil-specs
```
