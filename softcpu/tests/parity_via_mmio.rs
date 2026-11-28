//! 1C host-sim digest parity via MMIO — goldens from `softcpu/PARITY.md`.
//! Same fixtures as `tests/parity/test_digest_oracle.rs` (1A oracle stays untouched).

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::digest::sha256;
use bilkrnl_core::enclave::artifact_digest;
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::{ExitReason, FaultKind};
use softcpu::{blast_via_mmio, SoftCpu, CMD_BEGIN_LOAD, CMD_COMMIT_LOAD, CMD_RESET_ZEROIZE,
              CMD_RUN, LOAD_ERR_OK, OFF_CMD, OFF_IMG_LEN, OFF_LOAD_ERR, OFF_STATUS,
              OFF_ZERO_VERIFIED, STATUS_DISPOSED, STATUS_DONE, STATUS_FAULTED, STATUS_LOADED,
              STATUS_LOADING};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn hex32(s: &str) -> [u8; 32] {
    assert_eq!(s.len(), 64, "hex length");
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex");
    }
    out
}

struct Golden {
    exit: ExitReason,
    cycles: u64,
    out: &'static [u8],
    code_digest: &'static str,
    out_digest: &'static str,
    artifact_digest: &'static str,
}

fn image(code: &[u8], caps: Cap, cycles: u64, out: u32, pages: u16) -> BytecodeImage {
    BytecodeImage::new(0, code.to_vec(), caps, ResourceBudget::new(cycles, out, pages)).unwrap()
}

fn assert_golden(name: &str, img: BytecodeImage, g: &Golden) {
    let bil1 = img.encode();
    let r = blast_via_mmio(&bil1);
    assert_eq!(r.exit, g.exit, "{name}: exit");
    assert_eq!(r.cycles, g.cycles, "{name}: cycles");
    assert_eq!(r.out, g.out, "{name}: out");
    assert_eq!(r.code_digest, hex32(g.code_digest), "{name}: code_digest");
    assert_eq!(r.out_digest, hex32(g.out_digest), "{name}: out_digest");
    assert_eq!(r.out_digest, sha256(g.out), "{name}: out_digest = sha256(out)");
    assert_eq!(
        artifact_digest(&r),
        hex32(g.artifact_digest),
        "{name}: artifact_digest"
    );
}

#[test]
fn parity_via_mmio_halt() {
    let code = [0x02, 0x4f, 0x09, 0x02, 0x4b, 0x09, 0x02, 0x0a, 0x09, 0x00];
    assert_golden(
        "halt",
        image(&code, Cap::IoOutBounded, 64, 8, 1),
        &Golden {
            exit: ExitReason::Halt,
            cycles: 7,
            out: b"OK\n",
            code_digest: "0fe5a198c3048319789344d79fba856c45e178446346f1417fca0eed7cacf015",
            out_digest: "a12b7cb43c9d9134b5bb1b35e9096b66775d9e92e7611d1cc92b02edd6782a87",
            artifact_digest: "25351c265eba64629fc702c47cd44372035b3c53b6bf0a5b98f2e86cb1fe798d",
        },
    );
}

#[test]
fn parity_via_mmio_budget_exceed() {
    assert_golden(
        "budget_exceed",
        image(&[0x01, 0x01, 0x00], Cap::Compute, 2, 0, 1),
        &Golden {
            exit: ExitReason::BudgetExceeded,
            cycles: 3,
            out: b"",
            code_digest: "9f44ac6acb8a37b00b615dc89259d306035be82dee7db2b472e4c48326195440",
            out_digest: EMPTY_SHA256,
            artifact_digest: "f71a27b9d61afe8e0cd55e6cef3e2b7473567aa103a7a99ca15e0c2a85fd3600",
        },
    );
}

#[test]
fn parity_via_mmio_host_escape_e0() {
    assert_golden(
        "host_escape_e0",
        image(&[0xe0], Cap::IoOutBounded, 8, 0, 1),
        &Golden {
            exit: ExitReason::Fault(FaultKind::HostEscapeAttempt(0xE0)),
            cycles: 1,
            out: b"",
            code_digest: "7d8c5da7fd418379048e430b33dc8ffcda739e44326b8a5d647dc0ad81ed2157",
            out_digest: EMPTY_SHA256,
            artifact_digest: "23a1aa25890f331caef6315f58ae15868b873eeb6251be6c5811d88773ef52a5",
        },
    );
}

const OUT_FILL_CODE: [u8; 10] = [0x02, 0x58, 0x09, 0x02, 0x58, 0x09, 0x02, 0x58, 0x09, 0x00];

#[test]
fn parity_via_mmio_out_fill_ok() {
    assert_golden(
        "out_fill_ok",
        image(&OUT_FILL_CODE, Cap::IoOutBounded, 10000, 3, 1),
        &Golden {
            exit: ExitReason::Halt,
            cycles: 7,
            out: b"XXX",
            code_digest: "4664a3e53cb9aa682d22efe1845164d5b99d0275a93da5ea5b9101f849b3cec9",
            out_digest: "9b38b8f5877f2395b4361c1f68c059078ee9c0c8b0cbb22c97d2906e011e40a3",
            artifact_digest: "cb49179ea76e938d34735d1e889343a00c8bad911b555b550f3beb32062853e5",
        },
    );
}

#[test]
fn parity_via_mmio_out_fill_overflow() {
    assert_golden(
        "out_fill_overflow",
        image(&OUT_FILL_CODE, Cap::IoOutBounded, 10000, 2, 1),
        &Golden {
            exit: ExitReason::Fault(FaultKind::OutOverflow),
            cycles: 6,
            out: b"XX",
            code_digest: "4664a3e53cb9aa682d22efe1845164d5b99d0275a93da5ea5b9101f849b3cec9",
            out_digest: "ecc76c19c9f3c5108773d6c3a18a6c25c9bf1131c4e250b71213274e3b2b5d08",
            artifact_digest: "41b0e9d71ac13d020d637c4d6d3c195240e4521bd43403a680bbcefeb72110a4",
        },
    );
}

#[test]
fn parity_via_mmio_mem_oob() {
    assert_golden(
        "mem_oob",
        image(&[0x02, 0xab, 0x04, 0x00, 0x10, 0x00], Cap::Compute, 16, 0, 1),
        &Golden {
            exit: ExitReason::Fault(FaultKind::MemOob),
            cycles: 2,
            out: b"",
            code_digest: "1ebf65bafc3e70876abe010f188c5cac66883dca4f1bdbdfa45ae3f71cba05ea",
            out_digest: EMPTY_SHA256,
            artifact_digest: "9412dc121dfd93894348578d2203f00f9ba5a2545cd49e00f028fa741a9804a8",
        },
    );
}

#[test]
fn mmio_protocol_zero_verified_after_reset() {
    let img = image(
        &[0x02, 0x4f, 0x09, 0x02, 0x4b, 0x09, 0x02, 0x0a, 0x09, 0x00],
        Cap::IoOutBounded,
        64,
        8,
        1,
    );
    let bil1 = img.encode();
    let mut dev = SoftCpu::new(256);
    dev.write_u32(OFF_IMG_LEN, bil1.len() as u32);
    dev.write_u32(OFF_CMD, CMD_BEGIN_LOAD);
    assert_eq!(dev.read_u32(OFF_STATUS), STATUS_LOADING);
    dev.stream_image(&bil1);
    dev.write_u32(OFF_CMD, CMD_COMMIT_LOAD);
    assert_eq!(dev.read_u32(OFF_LOAD_ERR), LOAD_ERR_OK);
    assert_eq!(dev.read_u32(OFF_STATUS), STATUS_LOADED);
    dev.write_u32(OFF_CMD, CMD_RUN);
    let st = dev.read_u32(OFF_STATUS);
    assert!(st == STATUS_DONE || st == STATUS_FAULTED);
    // Snapshot digests before wipe (caller owns EnclaveResult).
    let _ = softcpu::enclave_result_from_mmio(&mut dev);
    dev.write_u32(OFF_CMD, CMD_RESET_ZEROIZE);
    assert_eq!(dev.read_u32(OFF_STATUS), STATUS_DISPOSED);
    assert_eq!(dev.read_u32(OFF_ZERO_VERIFIED), 1);
}
