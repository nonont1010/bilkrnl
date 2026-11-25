# Running

## Prerequisites

- Rust toolchain with `aarch64-unknown-uefi` target
- `qemu-system-aarch64`
- AAVMF firmware (`/usr/share/AAVMF/AAVMF_CODE.fd`, `AAVMF_VARS.fd`)
- Optional: `inkscape` or ImageMagick for PNG rasterization in docs refresh

```bash
rustup target add aarch64-unknown-uefi
sudo apt-get install -y qemu-system-arm qemu-efi-aarch64 inkscape
```

## One-shot verify (recommended)

```bash
./scripts/verify.sh --quick   # host tests + kernel build + docs refresh
./scripts/verify.sh           # + QEMU e2e, parity, perf, torture, docs refresh
```

## Manual QEMU

```bash
cargo build -p bilkrnl --target aarch64-unknown-uefi --release
./scripts/run-qemu.sh
cat target/qemu-serial.log
```

`run-qemu.sh` stages `bilkrnl.efi` as `ESP/EFI/BOOT/BOOTAA64.EFI`, copies AAVMF vars to `target/AAVMF_VARS.fd`, and boots `-machine virt -cpu cortex-a72 -nographic`.

### Environment

| Var | Meaning |
|-----|---------|
| `INTERACTIVE=1` | Foreground serial (no timeout capture) |
| `TIMEOUT_SEC` | Kill hung QEMU (default 30; verify uses 45) |
| `FIRMWARE_CODE` | Override UEFI code image |
| `BUILD_MODE` | `release` (default) or `debug` |

## Host-only (no QEMU)

```bash
cargo test -p bilkrnl-tests
```

Exercises the same contracts as the kernel without UEFI.
