# `bil1asm` — future host tool (sketch, not implemented)

**Status:** No code exists. This directory holds design notes and one reference image only. It is not a Cargo workspace member and is not run by `scripts/verify.sh`.

Design and phasing: [`docs/roadmap/b3-lowering.md`](../../docs/roadmap/b3-lowering.md).

## Hard boundary

- Host-only. Never linked into the UEFI `kernel` crate or into `bilkrnl-core`. If it ever needs `BytecodeImage::encode`, it may depend on `bilkrnl-core` (host target), never the other way round.
- Calls `bil` as a **subprocess** (`bil verify`, `bil vm --dump-bytecode`) and never as a Cargo dependency, to keep bil's Apache-2.0 sources and heavy deps out of the MIT workspace.
- Output images get no extra trust. They go through the same load → blast → dispose → `chamber_ratify` path as hand-written images.

## Interface (proposed)

```text
bil1asm assemble <prog.bil1.ir>  -o <out.bil1>      # B3.1: hand-written profile IR
bil1asm lower    <prog.bil> --entry <fn> -o <out.bil1>  # B3.2+: via bil stack bytecode
bil1asm dump     <img.bil1>                         # header + disassembly, for review
```

Inputs:

1. **Profile IR** (text, hand-written). One SoftISA-level statement per line, with symbolic labels and variable slots. This is the first milestone because it does not depend on bil's unstable dump format.
2. **bil stack bytecode** from `bil vm --dump-bytecode` (ghost-erased). Today this is Rust `Debug` text, so B3.2 should push for a stable `--format json` upstream before parsing it.
   `bil dump-ir --level lir` currently prints only signatures and is not usable as input.

Output: a `BIL1` image (32-byte header + code), byte-for-byte what `BytecodeImage::encode` would produce:

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | magic `"BIL1"` |
| 4 | 4 | `entry` (u32 LE) |
| 8 | 4 | `code_len` (u32 LE) |
| 12 | 1 | `initial_caps` (`0` None, `1` Compute, `2` IoOutBounded) |
| 13 | 3 | reserved, zero |
| 16 | 8 | `max_cycles` (u64 LE) |
| 24 | 4 | `max_out_bytes` (u32 LE) |
| 28 | 2 | `max_mem_pages` (u16 LE) |
| 30 | 2 | pad, zero |
| 32 | `code_len` | code |

## Op ↔ IR mapping

| SoftISA `Op` (byte) | Profile IR line | bil stack-bytecode source pattern |
|---------------------|-----------------|-----------------------------------|
| `LoadImm` (`02 kk`) | `ldi k` | `PushInt(k)` feeding ACC, `0 ≤ k ≤ 255` |
| `Load` (`03 lo hi`) | `ld x` | `LoadVar("x")` |
| `Store` (`04 lo hi`) | `st x` | `StoreVar("x")` / `AssignState([("x", …)])` |
| `AddImm` (`05 kk`) | `addi k` | `LoadVar`, `PushInt(k)`, `Binary(Add)` |
| `SubImm` (`06 kk`) | `subi k` | `LoadVar`, `PushInt(k)`, `Binary(Sub)` |
| `Jmp` (`07 lo hi`) | `jmp L` | back-edge of `GuardedLoop`, join of `GuardedSelect` |
| `Jz` (`08 lo hi`) | `jz L` | guard `x == 0` / `x > 0` (with `x ≥ 0`) / `x == k` (after `subi k`) |
| `Out` (`09`) | `out` | `Call { func: "bil1_out", argc: 1 }` (intrinsic) |
| `Weaken` (`0A cc`) | `weaken c` | `Call { func: "bil1_weaken", argc: 1 }` (intrinsic) |
| `Halt` (`00`) | `halt` | `Return` from the entry function |
| `Nop` (`01`) | `nop` | not emitted by lowering |
| `E0..=EF` | **never emitted** | — (assembler must refuse) |

Everything else (`Mul`/`Div`/`Mod`, var+var arithmetic, non-intrinsic `Call`, floats, strings, lists, structs, quantifiers, nondeterministic guards, `TrapAbort`) is rejected. The full list is in the roadmap.

## Concrete example: `fixture_ok_program`

Profile IR the tool would accept (B3.1):

```text
; ok.bil1.ir — reproduces bilkrnl_core::isa::fixture_ok_program
.caps   IoOutBounded
.budget cycles=64 out=8 pages=1
.entry  start
start:
    ldi 0x4f   ; 'O'
    out
    ldi 0x4b   ; 'K'
    out
    ldi 0x0a   ; '\n'
    out
    halt
```

Assembly, line by line:

| IR | Bytes |
|----|-------|
| `ldi 0x4f` | `02 4f` |
| `out` | `09` |
| `ldi 0x4b` | `02 4b` |
| `out` | `09` |
| `ldi 0x0a` | `02 0a` |
| `out` | `09` |
| `halt` | `00` |

Code (10 bytes): `02 4f 09 02 4b 09 02 0a 09 00`, so `code_digest = 0fe5a198c3048319789344d79fba856c45e178446346f1417fca0eed7cacf015`. That matches [`softcpu/PARITY.md`](../../softcpu/PARITY.md) vector 1. The full 42-byte image is in [`examples/ok.bil1.txt`](examples/ok.bil1.txt).

The matching bil-side oracle (the source that `lower` would eventually take) is a `fn` returning the expected `out` bytes, `[79, 75, 10]`. A lowering that calls `bil1_out` three times on those literals and then returns must produce the same 10 bytes.

## Acceptance for the first real implementation (B3.1)

- `bil1asm assemble ok.bil1.ir` output is byte-identical to `examples/ok.bil1.txt`.
- `bil1asm dump` on that image round-trips to the same IR (modulo comments).
- Running the image through `bilkrnl-core::isa::Vm::run` gives `Halt`, `out = "OK\n"`, and the `out_digest` / `artifact_digest` listed in `softcpu/PARITY.md`.
