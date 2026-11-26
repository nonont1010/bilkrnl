# QEMU `-device bil-softcpu` Registration (Design Notes)

> **DESIGN NOTES ONLY — NOT IMPLEMENTED.**
> No QEMU fork, board patch, or device `.c` lives in this repository. `scripts/verify.sh` and `scripts/run-qemu.sh` keep booting stock `qemu-system-aarch64 -machine virt` with no soft-CPU device. The C sketch [`include/bil_softcpu.h`](./include/bil_softcpu.h) is not compiled here. Register map: [`DEVICE.md`](./DEVICE.md). Digest oracle: [`PARITY.md`](./PARITY.md).

## Non-negotiable contracts

A future fork may add a device. It may not retune these against `bilkrnl-core`:

- `BytecodeImage` encode/decode (`BIL1`, 32-byte header: magic `BIL1`, `entry: u32`, `code_len: u32`, `initial_caps: u8` + 3B reserved, `max_cycles: u64`, `max_out_bytes: u32`, `max_mem_pages: u16` + 2B pad, then `code` up to 64 KiB)
- Opcode map in `bilkrnl-core::isa::Op`, including `0xE0..=0xEF` host-escape traps (`FaultKind::HostEscapeAttempt`). Those opcodes never call a QEMU host service or a UEFI protocol.
- `EnclaveResult` fields and `artifact_digest` framing (`exit_tag || cycles_le64 || out_len_le32 || out || code_digest || out_digest`). The device publishes `CODE_DIGEST` and `OUT_DIGEST` only. Ratify stays in `bilkrnl-core::chamber`.
- Blast chamber order: dispose (`CMD_RESET_ZEROIZE` / `bil_softcpu_reset_zeroize`) then `chamber_ratify`. Zeroize is not a digest input.
- Capability poset / ratchet (`Cap`, weaken-only: `None (0) < Compute (1) < IoOutBounded (2)`)

The 1A interpreter remains the oracle. The device matches it. The oracle is not edited to match the device.

## Where the fork would put the code

This tree does not gain a submodule or a `hw/` tree. A fork of QEMU (the binary `scripts/run-qemu.sh` invokes today is stock) would carry, outside `bilkrnl`:

| Fork path | Role |
| :--- | :--- |
| `hw/misc/bil-softcpu.c` | `SysBusDevice` realize, MMIO ops, step/run/zeroize |
| `include/hw/misc/bil_softcpu.h` | Copy of [`include/bil_softcpu.h`](./include/bil_softcpu.h) |
| `hw/misc/Kconfig` | `config BIL_SOFTCPU`, default `n`, selected only by the fork |
| `hw/arm/virt.c` | One allow-list entry so `-device bil-softcpu` is legal on `virt` |

`bilkrnl` stays a UEFI guest. It does not link `libqemu` and it does not grow a Meson target for this header.

## Type registration

User-facing name is the qdev type `bil-softcpu`, parent `TYPE_SYS_BUS_DEVICE`. Sketch the fork would compile (not this repo):

```text
TYPE_BIL_SOFTCPU = "bil-softcpu"
instance: SysBusDevice parent; MemoryRegion mmio (size BIL_SOFTCPU_MMIO_SIZE);
          uint16_t max_pages (default 256)
class:    realize maps the region with memory_region_init_io + sysbus_init_mmio
          user_creatable = true
          props: max-pages (uint16, default 256), mmio-base (uint64, optional pin)
type_init → type_register_static
```

`-device bil-softcpu` is rejected by today's `qemu-system-aarch64` because that type is not registered. V1 never passes the flag.

`realize` checks `max-pages` is in `1..=256` (1 MiB ceiling, `MAX_LINEAR_BYTES`). The MMIO ops are little-endian, aligned, and in-window. A miss returns a clean device error and does not touch host files, the ESP, or UEFI services.

No IRQ is required for the single-flight poll of `STATUS`. An IRQ added later must not change digests.

## `virt` board glue

`scripts/run-qemu.sh` boots `-machine virt -cpu cortex-a72 -m 256M` plus AAVMF pflash and a FAT ESP. On that board, measured from `qemu-system-aarch64 -machine virt,dumpdtb=…` on the host that runs those scripts:

- `virtio_mmio@a001000` already occupies guest physical `0x0a001000` (virtio-mmio transports at `0x0a000000`, stride `0x200`).
- Dynamic sysbus children hang off `platform-bus@c000000` (`compatible = "qemu,platform"`), window `0x0c000000` size `0x02000000`.

The `mmio-base=0x0a001000` token in [`README.md`](./README.md) is the property shape, not a free GPA. A fork that maps the 64 KiB aperture there aliases a virtio-mmio slot.

Glue that fits this board:

1. In the fork's `virt_machine_class_init`, call `machine_class_allow_dynamic_sysbus_dev(mc, "bil-softcpu")`. That is the allow-list `virt` already uses for platform-bus devices. Without it, `-device bil-softcpu` fails even after the type exists.
2. Let the platform bus place the region inside `0x0c000000`–`0x0e000000` and emit a child node. Do not also `sysbus_mmio_map` onto `0x0a001000`.
3. If the fork honors the `mmio-base` property, `realize` rejects a base outside that platform-bus window or overlapping a region already in the memory map. Re-dump the DTB when the forked QEMU version moves `base_memmap[]`.
4. DTB child: `compatible = "bil,softcpu"`, `reg` = the bus-assigned base and `0x10000`. The guest reads `reg`. It does not compile in a GPA, and `BIL1` does not carry one.

`max-pages=256` may be passed on the command line. Omitting it uses the default in the header (`BIL_SOFTCPU_DEFAULT_MAX_PAGES`).

V1 command line is unchanged. A later opt-in, only against the forked binary, appends `-device bil-softcpu,max-pages=256` to the existing `run-qemu.sh` argv (same pflash, same ESP, same `-machine virt`). `scripts/verify.sh` does not grow that flag. Its 1C check is still "roadmap file exists".

## ESP and kernel: same BIL1 load API

The ESP image stays `EFI/BOOT/BOOTAA64.EFI` copied from `bilkrnl.efi`. No new UEFI protocol, no second loader, no MMIO pointer on `enclave_load`.

`kernel/src/main.rs` still builds a `BytecodeImage` and calls `blast`. The public surface stays:

- `BytecodeImage::{new, encode, decode}`
- `enclave_load` / `EnclaveSlot::load` / `load_bytes`
- `enclave_run`, `enclave_dispose`, `blast`
- `chamber_ratify(&result, &spec)` and `artifact_digest(result)` on the host side of the chamber

A future slot-private probe, after the same load API has the image:

1. Locate the DTB. On AAVMF that is the EFI configuration table whose GUID is the device-tree table (`b1b621d5-f19c-41a5-830b-d9152c69aae0`). Missing table means the device is absent.
2. Find `compatible = "bil,softcpu"` and read `reg` as the MMIO base. Missing node means the device is absent.
3. Read `MAGIC` (`BIL_SOFTCPU_OFF_MAGIC`). Value `0x314C4942` selects the device backend for that boot. Anything else selects `bilkrnl-core::isa::Vm`.
4. Device path: `img.encode()` is `IMG_LEN` + `IMG_FIFO` + `CMD_COMMIT_LOAD`. The FIFO bytes are that vector. The device re-checks the header.
5. Interpreter path: `Vm::new(&img.code, img.entry, img.budget, img.initial_caps)` as today.
6. `run` returns `EnclaveResult` from `Vm::run` or from the result window in DEVICE.md §4. `dispose` is `Vm::zeroize` or `bil_softcpu_reset_zeroize`. `blast` gains no device argument.

Callers still pass a `BytecodeImage`. They do not pass an MMIO base.

## Interpreter fallback when the device is absent

Absence is the V1 path and must keep working after a fork exists.

| Probe result | Backend for that boot |
| :--- | :--- |
| No DTB table, no `bil,softcpu` node, MMIO window not mapped, or `MAGIC` ≠ `0x314C4942` | `isa::Vm`. Digests are the 1A results in [`PARITY.md`](./PARITY.md). |
| `MAGIC` matches | Device only. `COMMIT_LOAD` rejection returns `LoadError` (`Image` / `VmInit` / `Busy`). It is not replayed on `Vm`. |
| Device selected, then `RESET_ZEROIZE` | Same dispose-before-`chamber_ratify` order. `ZERO_VERIFIED == 1` and `STATUS == Disposed` before ratify. |

A missing device is a fallback. A failed `COMMIT_LOAD` after a successful probe is an error. Serial `chamber: RATIFIED <64-hex>` on the fallback path is the interpreter's `artifact_digest`. With the device present, the same marker must match that oracle byte-for-byte ([`PARITY.md`](./PARITY.md)).

## Explicitly out of V1 verify

- `scripts/verify.sh` does not compile `include/bil_softcpu.h`, does not configure QEMU, and does not pass `-device bil-softcpu`.
- No in-tree Meson, CMake, or Kconfig pulls this sketch into `bilkrnl` or `bilkrnl-core`.
- Stock `run-qemu.sh` remains the boot that V1 calls green.
