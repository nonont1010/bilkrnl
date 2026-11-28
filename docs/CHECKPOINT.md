---
status: DONE — B3.2 bil1asm lower + reject diagnostics on main
next: optional B3.3 bil-verify gate or B3.4 goldens; 1C last
depends: []
---

# Checkpoint — bilkrnl — 2026-11-27

**Status:** B3.2 landed on main. Safe to `/compact`.

## Done (with evidence)

- Bil reachability witnesses @ `c7f6eb8` (MemOob/PcOob/HostEscape LTL PROVED).
- **B3.2** (parallel agents → integrated):
  - `bil1asm lower` parses `bil vm --dump-bytecode` / `.bc.txt`
  - Reject diagnostics: Mul, var+var, foreign Call, float, Int>255, TrapAbort
  - Happy path → `02 4f 09 … 00` (`fixture_ok_program`)
  - Research: `docs/roadmap/b3.2-dump-notes.md` (Debug dump; no JSON yet)
  - Fixtures: `tools/bil1asm/examples/lower/`
  - `cargo test -p bil1asm` → **19 passed** (after out_hello file test)

## Durable decisions

- Dump format remains Rust Debug until upstream bil adds `--format json`; prefer checked-in `.bc.txt`.
- Live dumps with `let x = bil1_out(k)` emit StoreVar/PopVar — stripped profile fixtures are the digest source of truth.
- B3.2 stays out of V1 verify until dump format is stable (still in `cargo test -p bil1asm` / unit tier via workspace).
- Next: B3.3 (`bil verify` before lower) → B3.4 goldens → 1C last.

## Working-tree state

- Branch: `main`. bil: `/home/bill/src/lang/bil/target/release/bil` (banner may say 0.7.1; sources 0.8.1).

## Resume command

```bash
cd /home/bill/src/os/bilkrnl && cargo test -p bil1asm && ./scripts/verify.sh --quick
```
