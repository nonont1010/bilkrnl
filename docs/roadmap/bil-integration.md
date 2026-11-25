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
- [ ] Optional verify gate: if `bil` on `PATH`, `model-check` the spec (skip if absent)

### Phase B1 — SoftISA as `system`

Model `Vm::step` as guarded transitions (PC/ACC/mem/out/budget) with invariants:

- host opcodes `0xE0..=0xEF` never mutate a “host_touched” ghost flag
- `Out` requires `CapIoOut`
- cycle counter monotonic and ≤ `max_cycles` or exit `BudgetExceeded`

Prove alignment with Rust unit tests TC-03..08.

### Phase B2 — Chamber FOL

Encode `RatifySpec` as bil axioms; prove Reject paths for wrong exit / over-budget / cap ceiling (TC-13).

### Phase B3 — Lowering (stretch)

`bil dump-ir` / LIR → assembler emitting `BIL1` images; digests must match SoftISA runs (paves road to 1C soft-CPU with shared specs).

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
