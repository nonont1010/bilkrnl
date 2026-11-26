# Incorporating [bsommers/bil](https://github.com/bsommers/bil)

**bil** = *Bounded Invariant Logic* (Apache-2.0, Rust): guarded commands, `system` model checking, WP/`verify`, SMT (`z3`/`cvc5`/`bil-smt`), `emit-verus`, and a host bytecode VM / native `build`.

This is a strong fit for **bilkrnl** — same author lineage, complementary layers. bil does **not** replace the UEFI kernel or SoftISA runtime; it becomes the **host-side specification and proof toolchain** for the contracts bilkrnl executes.

## Fit map

| bil capability | bilkrnl seam | Incorporation |
|----------------|--------------|---------------|
| `system` + `invariant` + `model-check` | `EnclaveSlot` phases, dispose→ratify | Spec in [`specs/bil/enclave_chamber.bil`](../../specs/bil/enclave_chamber.bil) |
| `fn` + `ensures` + `verify` | `CapPoset::weaken`, budgets | Pure ratchet theorems in same `.bil` |
| Domain zero-trust / handshake examples | No host-escape, chamber distrust | Pattern for SoftISA adversary transitions |
| `emit-verus` | Future inductive proofs over Rust core | Optional CI when `verus` present |
| `vm` / `build` (host) | Reference semantics for high-level enclave programs | Later: lower bil LIR → `BIL1` (not V1) |
| OntoLogic / FOL policies | Cap lattice, RatifySpec predicates | Host audit of policy files before load |

## Hard boundary (do not violate)

- **Do not** link `bil` into the UEFI `kernel` crate. bil pulls `serde`/`serde_json`, SMT, and a heavy driver — wrong TCB shape for aarch64-unknown-uefi.
- bil stays **host-side** (CI / `scripts/verify.sh` optional tier / developer workstation).
- Runtime truth remains `bilkrnl-core` + QEMU evidence ([EVIDENCE.md](../EVIDENCE.md)).

## Phased plan

### Phase B0 (now)

- [x] Document this roadmap
- [x] Land `specs/bil/enclave_chamber.bil` modeling load/run/dispose/ratify + cap weaken
- [x] Optional verify gate: if `bil` on `PATH`, `model-check` the spec (skip if absent)

**B0 verify gate:** `scripts/verify.sh` (`run_bil_specs`, or `./scripts/verify.sh --only bil-specs`) runs `bil check` + `bil model-check` on `enclave_chamber.bil`, `softisa_step.bil`, and `chamber_ratify.bil`; skips cleanly when `bil` is not on `PATH` (or `BIL_BIN` / `/tmp/bil-clone`).

### Phase B1 — SoftISA as `system`

Model `Vm::step` as guarded transitions (PC/ACC/mem/out/budget) with invariants:

- host opcodes `0xE0..=0xEF` never mutate a “host_touched” ghost flag
- `Out` requires `CapIoOut`
- cycle counter monotonic and ≤ `max_cycles` or exit `BudgetExceeded`

Prove alignment with Rust unit tests TC-03..08.

**B1 checklist:**

- [x] Land [`specs/bil/softisa_step.bil`](../../specs/bil/softisa_step.bil) (bounded abstract `system`: cycles / out_len / caps / exit / `host_touched` ghost; invariants mapped to TC-03, 04, 07, 08, 09 in the header)
- [x] `bil check` + `bil model-check` pass (70 states pre-refine; 439 after refine)
- [x] Wire `softisa_step.bil` into `scripts/verify.sh` `run_bil_specs`
- [x] Refine abstraction (pc 0..3, acc 0/1, `pages_touched` vs `max_mem_pages=1`; fetch/Jmp/Jz → `FaultPcOob`, Load/Store past page 0 → `FaultMemOob`; TC-05/TC-07 in header)

### Phase B2 — Chamber FOL

Encode `RatifySpec` as bil axioms; prove Reject paths for wrong exit / over-budget / cap ceiling (TC-13).

- [x] [`specs/bil/chamber_ratify.bil`](../../specs/bil/chamber_ratify.bil): one pure `fn` + `ensures` per `ChamberReject` (ExitNotAllowed → CyclesOverBudget → OutTooLarge → CapCeilingViolated → OutPredicateFailed), `chamber_ratify` check order, abstract `artifact_digest` (Ratified only)
- [x] TC-13 witnesses: `tc13_ratify_ok`, `tc13_wrong_out_reject`, `tc13_wrong_exit_reject`, `tc13_cycles_over_reject`; axiom witnesses for OutTooLarge / CapCeilingViolated
- [x] `system RatifyChamber` (bounded adversarial inputs) model-checks reject ordering + digest-only-on-Ratified
- [x] `bil check` + `bil verify --allow-bounded` (24/24 z3-proved) + `bil model-check` pass
- [x] Add `chamber_ratify.bil` to the `bil-specs` gate in `scripts/verify.sh`
- [x] Rust unit tests for OutTooLarge / CapCeilingViolated (`tc13_out_too_large_reject`, `tc13_cap_ceiling_reject`)

### Phase B3 — Lowering (stretch)

`bil dump-ir` / LIR → assembler emitting `BIL1` images; digests must match SoftISA runs (paves road to 1C soft-CPU with shared specs).

**Status: design only. Not implemented, not in V1 verify.** Full notes: [b3-lowering.md](b3-lowering.md). Host tool sketch: [`tools/bil1asm/README.md`](../../tools/bil1asm/README.md).

- Only a small **BIL1 profile** of bil lowers: `u8`-bounded literals and variables, `x ± k`, `== 0` / `== k` guards, deterministic single-branch `do … od` with a `variant`, plus `bil1_out` / `bil1_weaken` intrinsics. Mul/Div/Mod, var+var arithmetic, calls, recursion, floats, strings, ADTs, quantifiers, and nondeterministic guards are rejected.
- `bil dump-ir --level lir` (v0.8.1) prints signatures only. The usable input today is `bil vm --dump-bytecode`, which is Rust `Debug` text, so the first step is a hand-written profile IR.
- Digest parity: lowered images must be byte-deterministic, run to the bil oracle's `out` on the 1A interpreter, and pass `chamber_ratify`. Their golden digests become extra 1C parity vectors. B3 adds no new digest definitions.
- The same hard boundary applies: the tool is host-only, calls `bil` as a subprocess, and is never linked into `kernel` or `bilkrnl-core`.

- [x] B3.0 design notes + reference image hex ([`tools/bil1asm/examples/ok.bil1.txt`](../../tools/bil1asm/examples/ok.bil1.txt))
- [ ] B3.1 host assembler for profile IR reproducing `fixture_ok_program` byte-for-byte
- [ ] B3.2–B3.4 lower bil stack bytecode, `bil verify` side conditions, golden vectors

## License note

bil is **Apache-2.0**; bilkrnl workspace is **MIT**. Depending on bil as a **path/git host tool** or vendoring Apache-licensed specs is fine; keep NOTICE if shipping bil sources. Prefer **subprocess CLI** over Cargo dep inside `bilkrnl-core`.

## Commands

```bash
# From a bil checkout (https://github.com/bsommers/bil):
cargo install --path .          # or: cargo run -p bil -- …
bil model-check /path/to/bilkrnl/specs/bil/enclave_chamber.bil
bil verify /path/to/bilkrnl/specs/bil/enclave_chamber.bil --allow-bounded

# From bilkrnl (optional gate):
./scripts/verify.sh --only bil-specs
```
