# QEMU `bil-softcpu` sources

C SoftISA + SysBus MMIO device for optional forked QEMU.

| File | Role |
| :--- | :--- |
| `bil_isa.{h,c}` | SoftISA (`Vm` port) |
| `sha256.{h,c}` | Digests |
| `bil-softcpu.c` | `-device bil-softcpu` |
| `bil_softcpu_regs.h` | MMIO offsets (keeps design-only `include/bil_softcpu.h` uncompiled) |
| `test_bil_isa.c` | PARITY harness |
| `meson.build.snippet` / `Kconfig.snippet` / `patches/` | Upstream glue |

```bash
make -C softcpu/qemu test-isa
../../scripts/build-qemu-bil-softcpu.sh
SOFTCPU=1 ../../scripts/run-qemu.sh
```
