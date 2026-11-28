# Evidence

What a green `./scripts/verify.sh` run proves — and what it does **not**.

## Proven when ALL GREEN

| Claim | Evidence |
|-------|----------|
| Kernel UEFI-boots on aarch64 virt | Serial contains `bilkrnl: ready` |
| Demo blast ratifies | `chamber: RATIFIED` + 64-hex digest (policy golden / TC-16) |
| Sealed attested jobs | `chamber:rev4:` / `chamber:rot4:` match pinned goldens |
| CLI uses same core path | `cli: RATIFIED` digest equals policy chamber digest (TC-02) |
| Cycle/out/mem budgets fault closed | TC-03, TC-04, TC-05 |
| Bad images rejected | TC-06 typed `ImageError` |
| Host-shaped opcodes cannot escape | TC-08 `HostEscapeAttempt`; mem unchanged |
| Caps only weaken | TC-09 `RatchetViolation` on strengthen |
| Dispose zeros RAM; idempotent | TC-14, TC-10 |
| Single-flight enclave | TC-11 `LoadError::Busy` |
| Kill mid-boot ≠ clean halt success | TC-12 no `bilkrnl: halt` after SIGKILL |
| Docs graphics regenerable | `render-graphics.py` after build |
| C SoftISA digest parity | `make -C softcpu/qemu test-isa` vs `PARITY.md` |
| Opt-in softcpu QEMU e2e | `SOFTCPU=1` → `softcpu: probed` + dual-path digests |

## Explicitly not proven (V1)

- Absence of host speculative side channels across dispose boundaries (accepted; SoftISA moves semantics when probed).
- Exhaustive formal proof of every SoftISA opcode (bil `softisa_step` / chamber specs cover selected invariants).
- Multi-core / multi-enclave isolation.
- Production EFI runtime command surface (gated later; not enclave-grantable).
- FPGA/RTL SoftISA backend.

## Artifact digests

`artifact_digest` = SHA-256 over exit tag + cycles LE + out length LE + out bytes + `code_digest` + `out_digest`. Stable across host tests and QEMU serial for the same program.
