# `bil-softcpu` Device Model Outline

> **Host sim implemented** (`softcpu` Rust crate / `cargo test -p softcpu`). **QEMU SoftISA device sources** in `softcpu/qemu/` (optional forked binary via `scripts/build-qemu-bil-softcpu.sh`; not required for stock verify). **RTL deferred.** Register offsets below are the live MMIO contract for the host simulator and the C SysBus device.

## Overview

`bil-softcpu` models a deterministic, non-speculative 6502-style soft processor matching `bilkrnl-core::isa::Vm` (`ACC: u8`, `PC: u32`, `CAPS: u8`, 4 KiB-paged linear RAM up to 256 pages / 1 MiB, and bounded output FIFO). The guest kernel (`bilkrnl.efi`) drives single-flight enclave lifecycles (`load` → `step`/`run` → read `EnclaveResult` → `reset`/zeroize → `chamber_ratify`) over MMIO.

---

## 1. MMIO & Config Fields

Proposed 64 KiB MMIO aperture (little-endian 32-bit/64-bit aligned accesses; unaligned or out-of-window accesses fault cleanly without host side effects):

| Offset | Name | Width | Access | Description |
| :--- | :--- | :---: | :---: | :--- |
| `0x0000` | `MAGIC` | 32b | RO | Constant `0x314C4942` (`"BIL1"` LE) for device presence probe |
| `0x0004` | `VERSION` | 32b | RO | Interface version (`0x0001_0000` reserved for 1C draft) |
| `0x0008` | `HW_CAPS` | 32b | RO | Max supported linear pages (`bits[15:0]`, default `256`), max code bytes (`bits[31:16]`, `65535` / 64 KiB) |
| `0x000C` | `STATUS` | 32b | RO | Device phase: `0=Idle/Zeroed`, `1=Loading`, `2=Loaded`, `3=Running`, `4=Halted/Done`, `5=Faulted`, `6=Disposed` |
| `0x0010` | `CMD` | 32b | WO | Command strobe (`1=BEGIN_LOAD`, `2=COMMIT_LOAD`, `3=STEP`, `4=RUN`, `5=RESET_ZEROIZE`) |
| `0x0014` | `LOAD_ERR` | 32b | RO | `ImageError` / `VmInit` diagnostic code if `COMMIT_LOAD` rejects image |
| `0x0018` | `IMG_LEN` | 32b | RW | Total `BIL1` byte length (`32 + code_len`) expected during `Loading` |
| `0x001C` | `IMG_FIFO` | 32b | WO | Byte/word write port streaming raw `BIL1` header + code bytes |
| `0x0020` | `EXIT_KIND` | 32b | RO | `ExitReason` tag (`0=Halt`, `1=BudgetExceeded`, `2=Fault`) |
| `0x0024` | `FAULT_DETAIL` | 32b | RO | `FaultKind` discriminant + offending byte/opcode (`IllegalOpcode(u8)` / `HostEscapeAttempt(u8)`) |
| `0x0028` | `CYCLES_LO` | 32b | RO | Executed cycles `[31:0]` (`u64` low word) |
| `0x002C` | `CYCLES_HI` | 32b | RO | Executed cycles `[63:32]` (`u64` high word) |
| `0x0030` | `MEM_HIGH_WATER` | 16b | RO | `mem_high_water` pages touched (`u16`) |
| `0x0032` | `CAPS_AT_EXIT` | 8b | RO | `Cap` value at exit (`0=None`, `1=Compute`, `2=IoOutBounded`) |
| `0x0034` | `OUT_LEN` | 32b | RO | Number of bytes emitted into bounded output buffer |
| `0x0038` | `OUT_READ_IDX` | 32b | RW | Cursor into output buffer (`0..OUT_LEN`) |
| `0x003C` | `OUT_DATA` | 8b/32b | RO | Reads byte at `OUT_READ_IDX` and auto-increments cursor |
| `0x0040..0x005F` | `CODE_DIGEST` | 256b | RO | SHA-256 over loaded `code` slice (`[u8; 32]`) |
| `0x0060..0x007F` | `OUT_DIGEST` | 256b | RO | SHA-256 over emitted `out` slice (`[u8; 32]`) |
| `0x0080` | `ZERO_VERIFIED` | 32b | RO | `1` iff linear RAM, `out` buffer, and architectural registers verified zero after `RESET_ZEROIZE` |

Symbolic offsets (design sketch). Every address is `mmio-base + OFF_*`. `mmio-base` comes from the device property or DTB; the kernel does not hard-code a guest physical address inside `BIL1`. Width and access match the table. Numeric values are the draft 1C map, not a promise that hardware already exists.

```text
BIL_SOFTCPU_MMIO_SIZE           = 0x10000

BIL_SOFTCPU_OFF_MAGIC           = 0x0000   # u32 RO  0x314C4942 ("BIL1" LE)
BIL_SOFTCPU_OFF_VERSION         = 0x0004   # u32 RO  0x0001_0000
BIL_SOFTCPU_OFF_HW_CAPS         = 0x0008   # u32 RO  pages[15:0], max code[31:16]
BIL_SOFTCPU_OFF_STATUS          = 0x000C   # u32 RO
BIL_SOFTCPU_OFF_CMD             = 0x0010   # u32 WO
BIL_SOFTCPU_OFF_LOAD_ERR        = 0x0014   # u32 RO
BIL_SOFTCPU_OFF_IMG_LEN         = 0x0018   # u32 RW
BIL_SOFTCPU_OFF_IMG_FIFO        = 0x001C   # u32 WO  low 8 bits are the next image byte
BIL_SOFTCPU_OFF_EXIT_KIND       = 0x0020   # u32 RO  same tags as artifact_bytes
BIL_SOFTCPU_OFF_FAULT_DETAIL    = 0x0024   # u32 RO
BIL_SOFTCPU_OFF_CYCLES_LO       = 0x0028   # u32 RO
BIL_SOFTCPU_OFF_CYCLES_HI       = 0x002C   # u32 RO
BIL_SOFTCPU_OFF_MEM_HIGH_WATER  = 0x0030   # u16 RO
BIL_SOFTCPU_OFF_CAPS_AT_EXIT    = 0x0032   # u8  RO
BIL_SOFTCPU_OFF_OUT_LEN         = 0x0034   # u32 RO
BIL_SOFTCPU_OFF_OUT_READ_IDX    = 0x0038   # u32 RW
BIL_SOFTCPU_OFF_OUT_DATA        = 0x003C   # u8/u32 RO
BIL_SOFTCPU_OFF_CODE_DIGEST     = 0x0040   # 32 bytes RO  sha256(code)
BIL_SOFTCPU_OFF_OUT_DIGEST      = 0x0060   # 32 bytes RO  sha256(out)
BIL_SOFTCPU_OFF_ZERO_VERIFIED   = 0x0080   # u32 RO

STATUS_IDLE = 0    STATUS_LOADING = 1   STATUS_LOADED = 2
STATUS_RUNNING = 3 STATUS_DONE = 4      STATUS_FAULTED = 5
STATUS_DISPOSED = 6

CMD_BEGIN_LOAD = 1  CMD_COMMIT_LOAD = 2  CMD_STEP = 3
CMD_RUN = 4         CMD_RESET_ZEROIZE = 5

EXIT_HALT = 0  EXIT_BUDGET_EXCEEDED = 1  EXIT_FAULT = 2
```

`FAULT_DETAIL` packs `FaultKind` so the kernel can rebuild `ExitReason::Fault` without a private encoding. Bits `[7:0]` are the payload byte (opcode, or `0` when the variant has none). Bits `[15:8]` are the discriminant, in `bilkrnl-core::isa::FaultKind` declaration order:

| Discriminant | Name | Payload |
| :---: | :--- | :--- |
| `0` | `IllegalOpcode` | opcode |
| `1` | `PcOob` | `0` |
| `2` | `MemOob` | `0` |
| `3` | `HostEscapeAttempt` | opcode in `0xE0..=0xEF` |
| `4` | `MissingCap` | `0` |
| `5` | `OutOverflow` | `0` |

`EXIT_KIND` values are the `artifact_bytes` tags (`0` Halt, `1` BudgetExceeded, `2` Fault). The kernel copies them through; it does not remap them.

QEMU device properties (`-device bil-softcpu,...`, design note):

- `mmio-base` (`uint64`): guest physical address on `virt` bus.
- `max-pages` (`uint16`, default `256`): hard linear RAM ceiling (`<= MAX_LINEAR_BYTES`, 1 MiB).

---

## 2. Load Bytecode (`BIL1`)

Mirrors `BytecodeImage::decode` + `Vm::new` without changing kernel-facing `enclave_load`:

1. **Require clean slot:** Device must be in `STATUS = Idle/Zeroed` (`0`) or `Disposed` (`6`); otherwise `BEGIN_LOAD` fails as busy (single-flight V1 invariant).
2. **Stage image:** Driver writes total length to `IMG_LEN`, strobes `CMD = BEGIN_LOAD`, and streams the raw `BIL1` image (32-byte header + `code`) into `IMG_FIFO`.
3. **Validate & initialize (`CMD = COMMIT_LOAD`):**
   - Check magic `BIL1`, `code_len <= 64 KiB`, `max_mem_pages > 0` and `<= max-pages`, `initial_caps in 0..=2`, and `entry < code_len` (when non-empty) and `code_len <= max_mem_pages * 4096`.
   - Allocate/clear `max_mem_pages * 4096` bytes inside device state, copy `code` into `mem[0..code_len]`, compute SHA-256 `CODE_DIGEST`, set `PC = entry`, `ACC = 0`, `CAPS = initial_caps`, `CYCLES = 0`, `MEM_HIGH_WATER = 0`, `OUT_LEN = 0`.
   - Transition `STATUS -> Loaded` (`2`), or set `LOAD_ERR` and remain non-runnable on validation failure.

---

## 3. Step / Run Execution

Mirrors `Vm::step` and `Vm::run` — direct architectural commit per instruction, no branch prediction, no speculative prefetch:

- **`CMD = STEP` (`3`):** Valid only in `Loaded` or `Running`. Executes exactly one `bilkrnl-core::isa::Op` instruction:
  1. Charge 1 cycle against `max_cycles`; if `cycles > max_cycles`, latch `EXIT_KIND = BudgetExceeded` (`1`) and transition `STATUS -> Halted/Done` (`4`).
  2. Verify `CAPS >= Cap::Compute`; otherwise latch `FaultKind::MissingCap` and transition `STATUS -> Faulted` (`5`).
  3. Fetch opcode at `PC` (checking `PC < mem_bytes` else `PcOob`).
  4. Reject `0xE0..=0xEF` immediately with `FaultKind::HostEscapeAttempt(op)` — **never** invoke QEMU host services or UEFI callbacks.
  5. Execute `Op::Halt..=Op::Weaken`, enforcing `mem_high_water` page accounting on `Load`/`Store`, `Cap::IoOutBounded` + `max_out_bytes` on `Out`, and monotone `CapPoset::weaken` on `Weaken`. Unknown opcodes fault with `IllegalOpcode(op)`.
- **`CMD = RUN` (`4`):** Valid in `Loaded`. Iterates `STEP` internally inside the QEMU device loop until `Halt`, `BudgetExceeded`, or `Fault(_)`, then computes `OUT_DIGEST = sha256(out)` before exposing `STATUS = Halted/Done` (`4`) or `Faulted` (`5`).

---

## 4. Read Result Registers

Once `STATUS` reaches `Halted/Done` (`4`) or `Faulted` (`5`), the kernel driver reconstructs `EnclaveResult` identically to 1A:

1. Read `EXIT_KIND` (+ `FAULT_DETAIL` when `EXIT_KIND == 2`) → `ExitReason`.
2. Read `CYCLES_LO` / `CYCLES_HI` → `cycles: u64`.
3. Read `MEM_HIGH_WATER` → `mem_high_water: u16` and `CAPS_AT_EXIT` → `caps_at_exit: Cap`.
4. Set `OUT_READ_IDX = 0` and read `OUT_LEN` bytes from `OUT_DATA` → `out: Vec<u8>`.
5. Read 32-byte `CODE_DIGEST` and `OUT_DIGEST` registers (and cross-check `sha256(&out) == OUT_DIGEST` on the kernel side as defense in depth).

The resulting `EnclaveResult` feeds `artifact_digest` and `chamber_ratify` unchanged.

---

## 5. Reset & Zeroize (Dispose Contract)

Mirrors `Vm::zeroize` and `EnclaveSlot::dispose` — mandatory **before** `chamber_ratify`:

`blast` order in `bilkrnl-core::enclave` is load → run → **return `EnclaveResult` to the caller** → dispose. `chamber_ratify` then hashes that returned value. Dispose drops slot-local state: `Phase::Ran` zeros the RAM clone and replaces the phase with `Disposed { zero_probe }`; `Phase::Loaded` calls `Vm::zeroize` first. The caller's `EnclaveResult` (including `out`, both digests, and `cycles`) is the only copy left. `RESET_ZEROIZE` follows that order. It is not a digest input.

1. Kernel writes `CMD = RESET_ZEROIZE` (`5`). Legal from `Loaded`, `Halted/Done`, or `Faulted` (software dispose accepts `Loaded` or `Ran`, and `Ran` covers halt, budget, and fault). From `Disposed`, the command is idempotent (`Dispose` on an already disposed slot returns `Ok`). From `Idle` with no image loaded, the device does not invent a disposed slot (`DisposeError::NotFound` analogue: stay `Idle`, `ZERO_VERIFIED` stays `0`).
2. Device synchronously overwrites, matching `Vm::zeroize` plus the `Phase::Ran` memory wipe:
   - Entire allocated linear RAM (`0..max_mem_pages * 4096`) with `0x00` (this is the `disposed_mem` / TC-14 probe).
   - Entire internal `out` buffer and staging `IMG_FIFO` buffer with `0x00`, then `OUT_LEN = 0` and `OUT_READ_IDX = 0`. A post-dispose `OUT_DATA` read must not return the pre-dispose output. The kernel already copied those bytes into its `EnclaveResult`.
   - Architectural state: `ACC = 0`, `PC = 0`, `CAPS = Cap::None (0)`, budget/cycle counters `= 0`, `MEM_HIGH_WATER = 0`.
   - Result-window leftovers the software slot drops with the `Vm`: `CODE_DIGEST`, `OUT_DIGEST`, `EXIT_KIND`, `FAULT_DETAIL`, and `CAPS_AT_EXIT` read back as zero. Clearing them is safe only because step 4 of "Read Result Registers" has already copied them out.
3. Device sweeps its backing buffers to assert all bytes equal `0x00`, sets `ZERO_VERIFIED = 1`, and transitions `STATUS -> Disposed` (`6`) (idempotent on repeat `RESET_ZEROIZE`).
4. Kernel checks `STATUS == Disposed && ZERO_VERIFIED == 1` before invoking `chamber_ratify(&result, &spec)`. Ratify stays in `bilkrnl-core::chamber`. There is no MMIO command for it. The device never sees `RatifySpec`.

`BEGIN_LOAD` remains allowed from `Disposed`, which is the software slot's rule that a disposed phase is not `Busy`.

---

## 6. Keeping the `BIL1` load API unchanged

Guest call sites keep today's signatures. `kernel/src/main.rs` still builds a `BytecodeImage` and calls `blast`; the CLI still talks to `EnclaveSlot`. No MMIO pointer, opcode, or digest layout is added to `enclave_load`.

Public surface that stays put:

- `BytecodeImage::{new, encode, decode}` — 32-byte `BIL1` header plus `code`
- `enclave_load` / `EnclaveSlot::load` / `load_bytes(bytes)`
- `enclave_run`, `enclave_dispose`, `blast` (load → run → dispose → `EnclaveResult`)
- `chamber_ratify(&result, &spec)` and `artifact_digest(result)` on the host side of the chamber

A future backend switch is private to the slot. Callers pass the same image either way:

```text
probe MAGIC at mmio-base
  missing or not 0x314C4942  →  EnclaveSlot uses isa::Vm     (1A, V1 default)
  MAGIC matches              →  EnclaveSlot streams encode() (1C backend)
```

`load` / `load_bytes` still decode to `BytecodeImage` in the kernel, then:

1. Interpreter backend: `Vm::new(&img.code, img.entry, img.budget, img.initial_caps)` as today.
2. Device backend: `img.encode()` is written to `IMG_LEN` + `IMG_FIFO` and `COMMIT_LOAD`. The bytes on the FIFO are the same vector `encode` already produces. The device re-checks the header; it does not accept a second image format.

`run` returns `EnclaveResult` built either by `Vm::run` or by reading the result window in section 4. `dispose` calls `Vm::zeroize` or `CMD_RESET_ZEROIZE`. `blast` does not grow a device argument. Probe failure at boot selects the interpreter for that boot; that is the V1 path, and the digests in [`PARITY.md`](./PARITY.md) are what it already produces. Once `MAGIC` has selected the device, a bad image returns `LoadError` (`Image` / `VmInit`) through the same `enclave_load` result type. It is not replayed on `Vm`. The device is required to match the 1A oracle. The oracle is not edited to match the device.
