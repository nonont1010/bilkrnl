# `bil1asm` — host BIL1 assembler + stack-bytecode lowerer (B3.1 / B3.2)

**Status:** Implemented. Workspace member `tools/bil1asm`. Host-only; never linked into the UEFI `kernel` crate. Covered by `cargo test -p bil1asm` (also via `scripts/verify.sh` unit tier).

Design and phasing: [`docs/roadmap/b3-lowering.md`](../../docs/roadmap/b3-lowering.md).

## Hard boundary

- Host-only. May depend on `bilkrnl-core` (host target); the kernel never depends on this tool.
- Calls `bil` as a **subprocess** only for `lower` on `.bil` inputs (`BIL_BIN`, `PATH`, or a sibling `lang/bil` release binary). `.bc.txt` works with no bil install.
- Never emits opcodes `0xE0..=0xEF`.
- Output images get no extra trust. They go through the same load → blast → dispose → `chamber_ratify` path as hand-written images.

## Interface

```text
bil1asm assemble <prog.bil1.ir>       -o <out.bil1>
bil1asm dump     <img.bil1>
bil1asm lower    <prog.bc.txt|.bil>   -o <out.bil1>
bil1asm --help
```

Exit codes: `0` ok, `1` assemble/lower/IO error, `2` usage.

```bash
cargo run -p bil1asm -- lower tools/bil1asm/examples/ok.bc.txt -o /tmp/ok.bil1
cargo run -p bil1asm -- dump /tmp/ok.bil1
```

## Profile IR (B3.1)

```text
.caps   IoOutBounded          ; None | Compute | IoOutBounded
.budget cycles=64 out=8 pages=1
.entry  start
.byte   0x2A                  ; optional data prefix (sealed into code_digest)
start:
    ldi 0x4f
    out
    halt
```

| SoftISA `Op` | IR |
|---|---|
| `LoadImm` | `ldi k` |
| `Load` / `Store` | `ld @addr` / `st @addr` (or label) |
| `AddImm` / `SubImm` | `addi k` / `subi k` |
| `Jmp` / `Jz` | `jmp L` / `jz L` |
| `Out` / `Weaken` / `Halt` / `Nop` | `out` / `weaken Cap` / `halt` / `nop` |

## Stack bytecode lower (B3.2)

Input is `bil vm --dump-bytecode` Debug text, or a checked-in `.bc.txt` dialect matching that shape. Optional wrapper directives:

```text
.caps IoOutBounded
.budget cycles=64 out=8 pages=1
.fn emit
0000: PushInt(79)
0001: Call { func: "bil1_out", argc: 1 }
0002: Return
```

**Lowers:** `PushInt(0..=255)`, `StoreVar`/`LoadVar`/`PopVar` (slots after `code_len`, sorted names → ascending `u16`), `Binary(Add|Sub)` with one literal operand → `AddImm`/`SubImm`, `Call("bil1_out"|"bil1_weaken")` → `Out`/`Weaken`, `Return` → `Halt`. Header defaults: `caps=IoOutBounded` if any `bil1_out` else `Compute`; `cycles=64 out=8 pages=1`.

**Rejects (diagnostic substring):** `Binary(Mul|Div|Mod)`, var+var arith, `Call` except the two intrinsics, floats/strings/lists/structs/`Match`/`Quantifier`, nondeterministic `GuardedSelect`, `GuardedLoop` (B3.2 subset), `TrapAbort`, `Int` outside `0..=255`.

## Examples

| File | Role |
|---|---|
| [`examples/ok.bil1.ir`](examples/ok.bil1.ir) | Reproduces `fixture_ok_program` byte-for-byte |
| [`examples/ok.bc.txt`](examples/ok.bc.txt) | Same image via stack-bytecode `lower` |
| [`examples/ok.bil1.txt`](examples/ok.bil1.txt) | Hex dump reference for the same image |
| [`examples/policy_pass.bil1.ir`](examples/policy_pass.bil1.ir) | Killer job — sealed policy gate → `PASS\n` |
| [`examples/policy_fail.bil1.ir`](examples/policy_fail.bil1.ir) | Negative twin → `FAIL\n` |

```bash
cargo run -p bil1asm -- assemble tools/bil1asm/examples/policy_pass.bil1.ir -o /tmp/policy_pass.bil1
cargo run -p bil1asm -- dump /tmp/policy_pass.bil1
```

## Acceptance

- [x] B3.1: `assemble ok.bil1.ir` matches `examples/ok.bil1.txt` bytes
- [x] B3.1: `policy_pass` / `policy_fail` IR match `bilkrnl_core::isa` fixtures
- [x] B3.2: `lower ok.bc.txt` matches the same golden code bytes
- [x] B3.2: ≥4 reject cases with clear diagnostics
