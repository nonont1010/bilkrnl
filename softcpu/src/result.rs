//! Rebuild `EnclaveResult` from the MMIO result window and compute `artifact_digest`.

use bilkrnl_core::cap::Cap;
use bilkrnl_core::enclave::artifact_digest;
use bilkrnl_core::isa::{EnclaveResult, ExitReason};

use crate::device::{unpack_fault, SoftCpu};
use crate::regs::*;

/// Read the post-`RUN` result window into an `EnclaveResult` (DEVICE.md §4).
///
/// Caller must leave `STATUS` in `DONE` or `FAULTED`. Digests come from the
/// device registers; `out` is streamed via `OUT_DATA` with `OUT_READ_IDX` reset.
pub fn enclave_result_from_mmio(dev: &mut SoftCpu) -> EnclaveResult {
    let status = dev.read_u32(OFF_STATUS);
    assert!(
        status == STATUS_DONE || status == STATUS_FAULTED,
        "enclave_result_from_mmio: STATUS={status} (need DONE/FAULTED)"
    );

    let exit_kind = dev.read_u32(OFF_EXIT_KIND);
    let exit = match exit_kind {
        EXIT_HALT => ExitReason::Halt,
        EXIT_BUDGET_EXCEEDED => ExitReason::BudgetExceeded,
        EXIT_FAULT => ExitReason::Fault(unpack_fault(dev.read_u32(OFF_FAULT_DETAIL))),
        other => panic!("unknown EXIT_KIND {other}"),
    };

    let cycles_lo = dev.read_u32(OFF_CYCLES_LO) as u64;
    let cycles_hi = dev.read_u32(OFF_CYCLES_HI) as u64;
    let cycles = cycles_lo | (cycles_hi << 32);

    let mem_high_water = dev.read_u16(OFF_MEM_HIGH_WATER);
    let caps_raw = dev.read_u8(OFF_CAPS_AT_EXIT);
    let caps_at_exit = Cap::from_u8(caps_raw).expect("CAPS_AT_EXIT");

    let out_len = dev.read_u32(OFF_OUT_LEN) as usize;
    dev.write_u32(OFF_OUT_READ_IDX, 0);
    let mut out = Vec::with_capacity(out_len);
    for _ in 0..out_len {
        out.push(dev.read_u8(OFF_OUT_DATA));
    }

    let mut code_digest = [0u8; 32];
    for (i, b) in code_digest.iter_mut().enumerate() {
        *b = dev.read_u8(OFF_CODE_DIGEST + i as u32);
    }
    let mut out_digest = [0u8; 32];
    for (i, b) in out_digest.iter_mut().enumerate() {
        *b = dev.read_u8(OFF_OUT_DIGEST + i as u32);
    }

    EnclaveResult {
        exit,
        out,
        cycles,
        mem_high_water,
        code_digest,
        out_digest,
        caps_at_exit,
    }
}

/// `artifact_digest` over an MMIO-rebuilt `EnclaveResult` (must match PARITY.md).
pub fn artifact_digest_from_mmio(dev: &mut SoftCpu) -> [u8; 32] {
    artifact_digest(&enclave_result_from_mmio(dev))
}

/// Full host protocol: BEGIN_LOAD → FIFO → COMMIT_LOAD → RUN → result → RESET_ZEROIZE.
pub fn blast_via_mmio(bil1: &[u8]) -> EnclaveResult {
    let mut dev = SoftCpu::new(DEFAULT_MAX_PAGES);
    assert!(dev.magic_probe());

    dev.write_u32(OFF_IMG_LEN, bil1.len() as u32);
    dev.write_u32(OFF_CMD, CMD_BEGIN_LOAD);
    assert_eq!(dev.read_u32(OFF_STATUS), STATUS_LOADING);

    dev.stream_image(bil1);
    dev.write_u32(OFF_CMD, CMD_COMMIT_LOAD);
    assert_eq!(
        dev.read_u32(OFF_LOAD_ERR),
        LOAD_ERR_OK,
        "COMMIT_LOAD failed"
    );
    assert_eq!(dev.read_u32(OFF_STATUS), STATUS_LOADED);

    dev.write_u32(OFF_CMD, CMD_RUN);
    let status = dev.read_u32(OFF_STATUS);
    assert!(
        status == STATUS_DONE || status == STATUS_FAULTED,
        "RUN left STATUS={status}"
    );

    let result = enclave_result_from_mmio(&mut dev);

    dev.write_u32(OFF_CMD, CMD_RESET_ZEROIZE);
    assert_eq!(dev.read_u32(OFF_STATUS), STATUS_DISPOSED);
    assert_eq!(dev.read_u32(OFF_ZERO_VERIFIED), 1);

    result
}
