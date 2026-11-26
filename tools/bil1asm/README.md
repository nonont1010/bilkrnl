# `bil1asm` — host BIL1 profile IR assembler (B3.1)

**Status:** Implemented. Workspace member `tools/bil1asm`. Host-only; never linked into the UEFI `kernel` crate. Covered by `cargo test -p bil1asm` (also via `scripts/verify.sh` unit tier).

Design and phasing: [`docs/roadmap/b3-lowering.md`](../../docs/roadmap/b3-lowering.md).

## Hard boundary

- Host-only. May depend on `bilkrnl-core` (host target); the kernel never depends on this tool.
- Calls `bil` as a **subprocess** only in later B3.2+ (`lower`). B3.1 is pure assemble/dump.
- Output images get no extra trust. They go through the same load → blast → dispose → `chamber_ratify` path as hand-written images.

## Interface

```text
bil1asm assemble <prog.bil1.ir>  -o <out.bil1>
bil1asm dump     <img.bil1>
bil1asm --help
```

Exit codes: `0` ok, `1` assemble/IO error, `2` usage.

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

`0xE0..=0xEF` are never emitted as opcodes.

## Examples

| File | Role |
|---|---|
| [`examples/ok.bil1.ir`](examples/ok.bil1.ir) | Reproduces `fixture_ok_program` byte-for-byte |
| [`examples/ok.bil1.txt`](examples/ok.bil1.txt) | Hex dump reference for the same image |
| [`examples/policy_pass.bil1.ir`](examples/policy_pass.bil1.ir) | Killer job — sealed policy gate → `PASS\n` |
| [`examples/policy_fail.bil1.ir`](examples/policy_fail.bil1.ir) | Negative twin → `FAIL\n` |

```bash
cargo run -p bil1asm -- assemble tools/bil1asm/examples/policy_pass.bil1.ir -o /tmp/policy_pass.bil1
cargo run -p bil1asm -- dump /tmp/policy_pass.bil1
```

## Acceptance (B3.1)

- [x] `assemble ok.bil1.ir` matches `examples/ok.bil1.txt` bytes
- [x] `policy_pass` / `policy_fail` IR match `bilkrnl_core::isa` fixtures
- [x] PASS path: Halt, `out=PASS\n`, `caps_at_exit=Compute`, golden digests in `softcpu/PARITY.md`
