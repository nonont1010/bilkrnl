# 1A ↔ 1C Digest Parity Fixture Plan (Design Notes)

> **DESIGN NOTES ONLY — NOT IMPLEMENTED.**
> No Rust tests, fixture blobs, QEMU device, or RTL are added by this file. `scripts/verify.sh` does not run a soft-CPU and does not require one. The 1A interpreter (`bilkrnl-core::isa::Vm`) is the oracle and the kernel fallback.

This plan is how a future Milestone 1C `bil-softcpu` proves it matches the in-kernel interpreter on the three blast-chamber digests. It does not relax the contracts in [`README.md`](./README.md) and [`docs/roadmap/1c-softcpu.md`](../docs/roadmap/1c-softcpu.md): same `BIL1` image, same opcode map (including `0xE0..=0xEF`), same `EnclaveResult` framing, same dispose-then-`chamber_ratify` order.

## What is compared

For one `BIL1` image, decode with `BytecodeImage::decode`, run on 1A (`Vm::run` or `blast`), and run on 1C (MMIO `BEGIN_LOAD` → `COMMIT_LOAD` → `RUN` → read result registers in [`DEVICE.md`](./DEVICE.md)). Compare **before** `dispose` / `RESET_ZEROIZE`:

| Field | Rule |
| :--- | :--- |
| `code_digest` | SHA-256 of the `code` slice only (not the 32-byte header) |
| `out_digest` | SHA-256 of the `out` bytes actually committed. Empty `out` is SHA-256(`""`) = `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `artifact_digest` | SHA-256 of `artifact_bytes`: `exit_tag \|\| cycles_le64 \|\| out_len_le32 \|\| out \|\| code_digest \|\| out_digest` |

`exit_tag` matches `EXIT_KIND`: `0` Halt, `1` BudgetExceeded, `2` any `Fault(_)`. Fault *kind* is not a separate field inside `artifact_bytes`. Fixtures still record `FAULT_DETAIL` so two faults are not treated as equal just because a digest collided.

`artifact_digest` is computed in `bilkrnl-core::enclave::artifact_digest` from the reconstructed `EnclaveResult`. The device publishes `CODE_DIGEST` and `OUT_DIGEST` only. It does not own ratification.

Digests are taken from the result snapshot. Zeroize (TC-14) is a separate post-dispose probe: linear RAM reads back `0x00`. It is not an input to these three hashes.

Cycle rule the device must copy, not "fix": `BudgetAccount::charge_cycle` increments `cycles` even when it returns `CyclesExceeded`. A budget fault therefore reports `used == max_cycles + 1`. `Out` charges the byte **before** `push`; overflow leaves the previous `out` unchanged.

## Concrete fixtures

Header fields below are the `BIL1` budget/caps that `BytecodeImage::new` would encode (`entry = 0` in every case). Opcode bytes are the `code` slice. Digest hex is the 1A oracle for those bytes and the cycle/out rules above.

### 1. `halt` — clean halt and bounded `Out`

Maps to `isa::fixture_ok_program` (kernel demo blast) and the halt half of TC-08.

- Caps `IoOutBounded` (`2`). Budget `max_cycles = 64`, `max_out_bytes = 8`, `max_mem_pages = 1`.
- Code (10 bytes): `02 4f 09 02 4b 09 02 0a 09 00`  
  `LoadImm 'O'`, `Out`, `LoadImm 'K'`, `Out`, `LoadImm '\n'`, `Out`, `Halt`.
- Expect `EXIT_KIND = Halt (0)`, `cycles = 7`, `out = 4f 4b 0a` (`OK\n`), `caps_at_exit = 2`.
- `code_digest` `0fe5a198c3048319789344d79fba856c45e178446346f1417fca0eed7cacf015`
- `out_digest` `a12b7cb43c9d9134b5bb1b35e9096b66775d9e92e7611d1cc92b02edd6782a87`
- `artifact_digest` `25351c265eba64629fc702c47cd44372035b3c53b6bf0a5b98f2e86cb1fe798d`

### 2. `budget_exceed` — halt not retired

Maps to TC-03 `tc03_budget_exceeded_when_over` (`tests/unit/test_budget.rs`).

- Caps `Compute` (`1`). Budget `max_cycles = 2`, `max_out_bytes = 0`, `max_mem_pages = 1`.
- Code (3 bytes): `01 01 00` (`Nop`, `Nop`, `Halt`).
- Two nops retire (`cycles` 1 then 2). The third `step` charges `cycles = 3`, which is `> max_cycles`, and returns `BudgetExceeded` **without** executing `Halt`.
- Expect `EXIT_KIND = BudgetExceeded (1)`, `cycles = 3`, `out` empty.
- `code_digest` `9f44ac6acb8a37b00b615dc89259d306035be82dee7db2b472e4c48326195440`
- `out_digest` `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- `artifact_digest` `f71a27b9d61afe8e0cd55e6cef3e2b7473567aa103a7a99ca15e0c2a85fd3600`

### 3. `host_escape_e0` — opcode `0xE0` traps inside the device

Maps to TC-08 (`tests/security/test_no_host_escape.rs`). `0xE1..=0xEF` are the same shape with a different opcode byte (different `code_digest`, fault payload, and `artifact_digest`). The named oracle is `0xE0` only.

- Caps `IoOutBounded` (`2`). Budget `max_cycles = 8`, `max_out_bytes = 0`, `max_mem_pages = 1`.
- Code (1 byte): `e0`.
- Expect `EXIT_KIND = Fault (2)`, `FAULT_DETAIL` discriminant `HostEscapeAttempt` payload `0xE0`, `cycles = 1`, `out` empty.
- Side condition (not hashed): `mem[0]` stays `0xE0`. No QEMU host call, no UEFI service, no `out` write.
- `code_digest` `7d8c5da7fd418379048e430b33dc8ffcda739e44326b8a5d647dc0ad81ed2157`
- `out_digest` `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- `artifact_digest` `23a1aa25890f331caef6315f58ae15868b873eeb6251be6c5811d88773ef52a5`

### 4. `out_fill` — exact fill, then one-past overflow

Maps to TC-04 (`tests/unit/test_outbuf.rs`). Two images share the code bytes and differ only in `max_out_bytes`.

Code (10 bytes): `02 58 09 02 58 09 02 58 09 00`  
three times (`LoadImm 'X'`, `Out`), then `Halt`. Caps `IoOutBounded`. `max_cycles = 10000`, `max_mem_pages = 1`.

**`out_fill_ok`** (`max_out_bytes = 3`): all three `Out`s commit, then `Halt`.

- `EXIT_KIND = Halt (0)`, `cycles = 7`, `out = 58 58 58` (`XXX`).
- `code_digest` `4664a3e53cb9aa682d22efe1845164d5b99d0275a93da5ea5b9101f849b3cec9`
- `out_digest` `9b38b8f5877f2395b4361c1f68c059078ee9c0c8b0cbb22c97d2906e011e40a3`
- `artifact_digest` `cb49179ea76e938d34735d1e889343a00c8bad911b555b550f3beb32062853e5`

**`out_fill_overflow`** (`max_out_bytes = 2`): the third `Out` fails `charge_out` before `push`.

- `EXIT_KIND = Fault (2)`, `FAULT_DETAIL = OutOverflow`, `cycles = 6`, `out = 58 58` (`XX`).
- `code_digest` same as `out_fill_ok`.
- `out_digest` `ecc76c19c9f3c5108773d6c3a18a6c25c9bf1131c4e250b71213274e3b2b5d08`
- `artifact_digest` `41b0e9d71ac13d020d637c4d6d3c195240e4521bd43403a680bbcefeb72110a4`

### 5. `mem_oob` — `Store` past the one mapped page

Maps to TC-05 `tc05_store_oob_fault` (`tests/unit/test_linear_as.rs`).

- Caps `Compute`. Budget `max_cycles = 16`, `max_out_bytes = 0`, `max_mem_pages = 1` (4096 bytes).
- Code (6 bytes): `02 ab 04 00 10 00`  
  `LoadImm 0xAB`, `Store` to `u16` `0x1000` (page 1), `Halt` not reached.
- `note_addr` sees page `1` so `pages_touched = 2 > max_mem_pages`. `MemOob` is raised before the store writes. `mem_high_water` stays `0`.
- Expect `EXIT_KIND = Fault (2)`, `FAULT_DETAIL = MemOob`, `cycles = 2`, `out` empty.
- `code_digest` `1ebf65bafc3e70876abe010f188c5cac66883dca4f1bdbdfa45ae3f71cba05ea`
- `out_digest` `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- `artifact_digest` `9412dc121dfd93894348578d2203f00f9ba5a2545cd49e00f028fa741a9804a8`

## `tests/` layout

**Now exists (1A-only oracle):** `tests/parity/test_digest_oracle.rs`, wired in `tests/Cargo.toml` as `test_digest_oracle`. It runs all six fixtures above through `Vm::run` and `blast` (after `BIL1` encode/decode) and asserts `code_digest`, `out_digest`, and `artifact_digest` against the hex in this file. It needs no 1C device and no QEMU. Run: `cargo test -p bilkrnl-tests --test test_digest_oracle`. It is not wired into `scripts/verify.sh`.

Still future (1C): the opt-in device suite below. Do not wire it into `scripts/verify.sh` until a device exists; V1 must keep passing with the interpreter only.

```text
tests/parity/                      # future crate or module; absent in V1
  fixtures/
    halt.bil1                      # raw BIL1 bytes for the cases above
    budget_exceed.bil1
    host_escape_e0.bil1
    out_fill_ok.bil1
    out_fill_overflow.bil1
    mem_oob.bil1
    expected.json                  # exit tag, fault detail, cycles, out hex, three digests
  parity_1a.rs                     # decode fixture → Vm::run → assert expected.json
  parity_1c.rs                     # same bytes via bil-softcpu MMIO; skip if MAGIC probe fails
```

`parity_1a.rs` is superseded by `test_digest_oracle.rs` and must keep agreeing with the hex in this file. `parity_1c.rs` stays skipped while the device is absent, which is the fallback path. Existing unit tests (TC-03, TC-04, TC-05, TC-08) stay where they are; this suite adds digest oracles, it does not replace them.

Host check, later: `sha256(artifact_bytes(result))` from 1A equals the same call on the `EnclaveResult` rebuilt from `EXIT_KIND`, `CYCLES_*`, `OUT_DATA`, `CODE_DIGEST`, and `OUT_DIGEST`. QEMU serial `chamber: RATIFIED <64-hex>` for the `halt` fixture must equal that `artifact_digest` under both backends.
