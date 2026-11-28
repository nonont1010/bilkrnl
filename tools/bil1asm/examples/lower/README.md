# B3.2 lowering fixtures (`bil` stack bytecode → BIL1)

Inputs for `bil1asm lower`. Design: [`docs/roadmap/b3-lowering.md`](../../../../docs/roadmap/b3-lowering.md). Dump research: [`docs/roadmap/b3.2-dump-notes.md`](../../../../docs/roadmap/b3.2-dump-notes.md).

## Format

Fixtures mirror `bil vm --dump-bytecode` Rust `Debug` lines, plus optional bil1asm directives:

```text
.caps IoOutBounded
.budget cycles=64 out=8 pages=1
.fn name():
  0000: PushInt(79)
  0001: Call { func: "bil1_out", argc: 1 }
  ...
```

| Directive | Notes |
|-----------|--------|
| Opcode lines | Real dump shape (`NNNN: Variant(...)`) |
| `.caps` / `.budget` | bil1asm-only (bil does not emit them). Prefer real lines over comments. |
| Banner / `.global` | Present in `out_hello.bil.bc.txt` (live dump); omitted from hand-written profile slices |

```bash
cargo run -p bil1asm -- lower tools/bil1asm/examples/lower/out_hello.bc.txt -o /tmp/out_hello.bil1
bil vm tools/bil1asm/examples/lower/out_hello.bil --dump-bytecode   # capture live dump
```

## Files

| File | Expect |
|------|--------|
| [`out_hello.bc.txt`](out_hello.bc.txt) | **lower OK** → SoftISA `02 4f 09 02 4b 09 02 0a 09 00` |
| [`out_hello.bil`](out_hello.bil) | bil source (identity stub `bil1_out`); `bil check` / `bil vm` OK |
| [`out_hello.bil.bc.txt`](out_hello.bil.bc.txt) | Live dump (stub + `StoreVar`/`PopVar` + `.global`); lowers with extra stores — prefer stripped `out_hello.bc.txt` for digests |
| [`reject_mul.bc.txt`](reject_mul.bc.txt) | **reject** — `Binary(Mul)` |
| [`reject_var_add.bc.txt`](reject_var_add.bc.txt) | **reject** — var+var `Binary(Add)` |
| [`reject_call.bc.txt`](reject_call.bc.txt) | **reject** — foreign `Call` |
| [`reject_float.bc.txt`](reject_float.bc.txt) | **reject** — `PushFloat` |
| [`reject_big_int.bc.txt`](reject_big_int.bc.txt) | **reject** — `PushInt(300)` |
