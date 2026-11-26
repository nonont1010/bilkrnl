# Phase B3 — Lowering bil → `BIL1` (stretch, design only)

**Status:** Design notes only. Nothing here is implemented. Not part of V1 verify gates. No assembler exists in-tree; [`tools/bil1asm/README.md`](../../tools/bil1asm/README.md) is a sketch of a future host tool.

Parent roadmap: [bil-integration.md](bil-integration.md) (B0–B2 are the spec/proof tiers; B3 is the only phase that would *produce* enclave code from bil).

## Hard boundary (restated, unchanged)

- **Do not** link `bil` into the UEFI `kernel` crate or into `bilkrnl-core`. The lowering tool is a **host-side** program that writes `BIL1` bytes to a file; the kernel only ever sees those bytes via the existing `BytecodeImage::decode` path.
- The runtime contract is fixed by `bilkrnl-core`: `BytecodeImage` (`BIL1`, 32-byte header), `isa::Op`, `EnclaveResult`, `artifact_digest`, `chamber_ratify`. B3 adapts to them; it never changes them.
- An image produced by B3 gets **no extra trust**. It is loaded, budgeted, blasted, disposed, and ratified exactly like a hand-assembled image. A bil proof about the *source* is advisory evidence, not a substitute for ratification.

## Inputs (what bil actually exposes today)

Probed with `bil v0.8.1` (`/tmp/bil-clone/target/release/bil`):

- `bil dump-ir --level lir` prints **signatures only** (`exec_fn emit_ok() -> Int`). It has no instruction bodies, so it is not a lowering input on its own.
- `bil vm --dump-bytecode` prints the ghost-erased **stack VM** program (`BytecodeOp` in bil's `src/vm.rs`): `PushInt`, `LoadVar`/`StoreVar`/`PopVar`, `Binary(op)`, `Unary(op)`, `Call`, `GuardedSelect`, `GuardedLoop`, `Match`, `Return`, plus list/struct/string/float/quantifier ops. It is Rust `Debug` text, so it is not a stable format.

Example (`fn emit_ok() -> Int = let a = 79 in let b = 75 in a + b`):

```text
.fn emit_ok():
  0000: PushInt(79)
  0001: StoreVar("a")
  0002: PushInt(75)
  0003: StoreVar("b")
  0004: LoadVar("a")
  0005: LoadVar("b")
  0006: Binary(Add)
  0007: PopVar("b")
  0008: PopVar("a")
  0009: Return
```

Consequence: B3 needs either (a) an upstream bil change that emits a stable machine-readable LIR/bytecode (e.g. `--format json`), or (b) a small, hand-written **BIL1 profile IR** that the tool accepts directly. Phase B3.0 below starts with (b) so nothing depends on bil internals.

## Target machine recap (`bilkrnl-core::isa`)

Single 8-bit accumulator, 32-bit PC, linear RAM of `max_mem_pages × 4096` bytes with code at address 0, bounded `out` buffer, cap ratchet. Operands are `imm8` or little-endian `u16` addresses.

| Op | Byte | Operand | Effect |
|----|------|---------|--------|
| `Halt` | `00` | — | exit `Halt` |
| `Nop` | `01` | — | — |
| `LoadImm` | `02` | imm8 | `ACC ← imm` |
| `Load` | `03` | u16 | `ACC ← mem[a]` |
| `Store` | `04` | u16 | `mem[a] ← ACC` |
| `AddImm` | `05` | imm8 | `ACC ← ACC + imm` (wrapping) |
| `SubImm` | `06` | imm8 | `ACC ← ACC − imm` (wrapping) |
| `Jmp` | `07` | u16 | `PC ← a` |
| `Jz` | `08` | u16 | if `ACC == 0` then `PC ← a` |
| `Out` | `09` | — | append `ACC` to `out` (needs `IoOutBounded`) |
| `Weaken` | `0A` | imm8 | `caps ← weaken(caps, imm)` |
| (trap) | `E0..=EF` | — | `Fault(HostEscapeAttempt)` |

There is no register file, no stack, no call/return, no carry/sign flag, and no memory-operand arithmetic.

## Construct mapping

### Lowers (BIL1 profile)

| bil construct / stack op | SoftISA sequence | Conditions |
|--------------------------|------------------|------------|
| `let x = k in …` → `PushInt(k)`, `StoreVar("x")` | `LoadImm k` · `Store @x` | `0 ≤ k ≤ 255`; `@x` is a fixed slot address assigned by the tool |
| `LoadVar("x")` (as next ACC value) | `Load @x` | — |
| `x + k` / `x - k` → `LoadVar`, `PushInt(k)`, `Binary(Add/Sub)` | `Load @x` · `AddImm k` / `SubImm k` | `k` is a literal; wrapping must be proven absent or intended (see "Integer width") |
| `x == k` as a branch guard | `Load @x` · `SubImm k` · `Jz L` | `k` literal |
| `x == 0` / `x > 0` guard on a value proven `≥ 0` | `Load @x` · `Jz L` | non-negativity from `requires` / `invariant` |
| `GuardedSelect` with deterministic, mutually exclusive guards of the shapes above | chain of `Jz` + `Jmp` | guards must be disjoint and exhaustive (bil `verify` can prove this) |
| `GuardedLoop` (`do … od`) with one branch `x > 0 -> x := x - k` | `L: Load @x` · `Jz END` · `SubImm k` · `Store @x` · `Jmp L` · `END:` | `variant` present (gives the cycle bound) |
| `x := e` (`AssignState`) where `e` is one of the forms above | compute `e` into ACC · `Store @x` | single-variable assignment |
| Byte output | `Out` | intrinsic, see below |
| Capability drop | `Weaken c` | intrinsic, `c ∈ {0,1,2}` |
| End of program / `Return` from the entry fn | `Halt` | entry function only |
| Ghost items (`requires`, `ensures`, `invariant`, `variant`, `spec`, `proof`, `axiom`) | nothing | already erased by bil; used only by the tool for static checks |

**Intrinsics.** bil has no I/O effect. The profile reserves two uninterpreted extern-style functions the tool recognizes by name: `bil1_out(v)` → `Out` and `bil1_weaken(c)` → `Weaken c`. The bil-side oracle for a lowered program is an `fn` that returns the intended `out` bytes as a `List<Int>`, so `bil vm` and the SoftISA run can be compared.

### Does not lower (reject with a diagnostic)

- `Binary(Mul | Div | Mod)`, any `Binary` with two non-literal operands (`a + b` with both variables), `Lt/Le/Ge` that need a sign or carry flag.
- `Call` to anything except the two intrinsics: no stack, no return address. Non-recursive helpers *could* be inlined later; recursion never lowers.
- Integers outside `0..=255` without a proof that they stay in range.
- `PushFloat`, `PushStr`, `MakeList`, `MakeStruct`, `UpdateStruct`, `GetField`, `Match` on ADTs, `PipeStage`, `Quantifier` (runtime `forall`/`exists`).
- Nondeterministic `GuardedSelect` / `GuardedLoop` (overlapping guards). SoftISA is deterministic, so the tool would silently pick one branch.
- `system` models. They describe transition relations for `model-check`, not a single program.
- `TrapAbort` (`CompiledAction`). There is no "abort" opcode. Emitting an `E0..=EF` byte to fake one is forbidden.

### Integer width

bil `Int` is unbounded (`i64` in the VM). SoftISA `ACC` is `u8` and wraps. Lowering is only sound when every lowered variable carries a `requires` / `invariant` bound of `0..=255` and `bil verify` proves it. Without that proof the tool must reject, not truncate.

## Emission rules (must hold for every emitted image)

1. **Deterministic bytes.** Same input → byte-identical image on any host. No hash-map iteration order, no timestamps. `code_digest = sha256(code)` depends on it.
2. **Never emit `0xE0..=0xEF` in opcode position.** They are legal as data bytes (`imm8`), but an assembler that emits them as opcodes is a bug, not a feature (TC-08).
3. **Jump targets land on instruction boundaries** and are `< code_len`.
4. **Variable slots live after code** (`code_len ≤ @x < max_mem_pages × 4096`). The tool never stores into the code region (no self-modifying code).
5. **Header comes from the source, not from defaults.** `initial_caps` is the minimal cap the program needs (`IoOutBounded` only if it uses `bil1_out`); `max_cycles` is derived from straight-line length plus `variant` bounds; `max_out_bytes` from the oracle list length; `max_mem_pages = 1` unless slots overflow.

## Digest-parity requirement (with 1A)

The 1A interpreter (`bilkrnl-core::isa::Vm::run`) stays authoritative. For every lowered program P:

1. **Bytes.** `sha256(code)` of the tool's output is stable across runs and recorded next to the source as a golden value.
2. **Semantics.** Running the image through 1A gives `exit = Halt` and `out` equal to the bil oracle's `List<Int>`, hence the expected `out_digest`.
3. **Ratification.** The full `EnclaveResult` passes `chamber_ratify` with a `RatifySpec` derived from the same source, and `artifact_digest` is recorded as a golden value.
4. **No new digest definitions.** B3 adds no hash and changes no field order; it only produces inputs to the existing ones.

The reference case is `fixture_ok_program`: the hand-written profile program in [`tools/bil1asm/README.md`](../../tools/bil1asm/README.md) must assemble to exactly `02 4f 09 02 4b 09 02 0a 09 00` (`code_digest 0fe5a198…cacf015`, see [`softcpu/PARITY.md`](../../softcpu/PARITY.md)). The image hex is in [`tools/bil1asm/examples/ok.bil1.txt`](../../tools/bil1asm/examples/ok.bil1.txt).

## Relationship to 1C soft-CPU

- 1C ([1c-softcpu.md](1c-softcpu.md)) swaps the *executor*; B3 adds a *producer*. They share only the `BIL1` contract.
- 1C's acceptance is "same digests on 1A and 1C for the fixture suite". B3 extends that suite: once lowered images have golden `code_digest` / `out_digest` / `artifact_digest` values, they become extra 1C parity vectors in `softcpu/PARITY.md` style with no change to 1C itself.
- Neither depends on the other. B3 can land before 1C (1A only), and 1C does not need B3.
- The B1 spec `specs/bil/softisa_step.bil` is the shared semantic reference (pc/acc/pages_touched refine landed). Both lowering rules and a future 1C device are checked against it.

## Phased plan

| Step | Deliverable | Gate |
|------|-------------|------|
| B3.0 | This doc + [`tools/bil1asm/README.md`](../../tools/bil1asm/README.md) + reference hex | docs only |
| B3.1 | Host tool assembles a hand-written **profile IR** (text) to `BIL1`; reproduces `fixture_ok_program` byte-for-byte | golden-byte test in the tool's own crate, outside `kernel` / `bilkrnl-core` |
| B3.2 | Accept bil stack bytecode for the profile subset. Prefer an upstream stable dump format over parsing `Debug` text | reject list above enforced with diagnostics |
| B3.3 | Range/determinism side conditions checked by running `bil verify` on the source first; refuse to lower on failure | — |
| B3.4 | Lowered programs added as 1A golden vectors (and 1C parity vectors once 1C exists) | optional tier, skipped when `bil` is absent |

## Explicitly out of V1 verify

- No assembler, lowering pass, or B3 test runs in `scripts/verify.sh`.
- No `tools/` crate is added to the Cargo workspace for V1.
- No B3 output is loaded by the kernel demo or any `tests/` case.
- Hand-assembled images (`fixture_ok_program` and friends) remain the only enclave code in V1.
