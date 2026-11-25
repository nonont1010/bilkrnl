# Evidence

What a green `./scripts/verify.sh` run proves — and what it does **not**.

## Proven when ALL GREEN

| Claim | Evidence |
|-------|----------|
| Kernel UEFI-boots on aarch64 virt | Serial contains `bilkrnl: ready` |
| Demo blast ratifies | `chamber: RATIFIED` + 64-hex digest |
| CLI uses same core path | `cli: RATIFIED` digest equals chamber digest (TC-02) |
| Cycle/out/mem budgets fault closed | TC-03, TC-04, TC-05 |
| Bad images rejected | TC-06 typed `ImageError` |
| Host-shaped opcodes cannot escape | TC-08 `HostEscapeAttempt`; mem unchanged |
| Caps only weaken | TC-09 `RatchetViolation` on strengthen |
| Dispose zeros RAM; idempotent | TC-14, TC-10 |
| Single-flight enclave | TC-11 `LoadError::Busy` |
| Kill mid-boot ≠ clean halt success | TC-12 no `bilkrnl: halt` after SIGKILL |
| Docs graphics regenerable | `render-graphics.py` after build |

## Explicitly not proven (V1)

- Absence of host speculative side channels across dispose boundaries (accepted; 1C hardens).
- Formal verification of the SoftISA.
- Multi-core / multi-enclave isolation.
- Production EFI runtime command surface (gated later; not enclave-grantable).

## Artifact digests

`artifact_digest` = SHA-256 over exit tag + cycles LE + out length LE + out bytes + `code_digest` + `out_digest`. Stable across host tests and QEMU serial for the same program.
