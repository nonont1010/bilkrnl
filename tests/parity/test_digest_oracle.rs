//! 1A digest-parity oracle: goldens from `softcpu/PARITY.md`.
//! Interpreter only (no 1C device, no QEMU).

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::digest::sha256;
use bilkrnl_core::enclave::{artifact_digest, blast, EnclaveSlot};
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::{EnclaveResult, ExitReason, FaultKind, Vm};

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

fn check(name: &str, img: BytecodeImage, g: &Golden) {
    // Direct Vm::run
    let mut vm = Vm::new(&img.code, img.entry, img.budget, img.initial_caps).unwrap();
    assert_digests(name, "Vm::run", &vm.run(), g);

    // Wire format round-trip, then blast pipeline.
    let decoded = BytecodeImage::decode(&img.encode()).unwrap();
    assert_eq!(decoded, img, "{name}: encode/decode");
    let mut slot = EnclaveSlot::new();
    let r = blast(&mut slot, decoded).unwrap();
    assert_digests(name, "blast", &r, g);
}

fn assert_digests(name: &str, via: &str, r: &EnclaveResult, g: &Golden) {
    assert_eq!(r.exit, g.exit, "{name}/{via}: exit");
    assert_eq!(r.cycles, g.cycles, "{name}/{via}: cycles");
    assert_eq!(r.out, g.out, "{name}/{via}: out");
    assert_eq!(r.code_digest, hex32(g.code_digest), "{name}/{via}: code_digest");
    assert_eq!(r.out_digest, hex32(g.out_digest), "{name}/{via}: out_digest");
    assert_eq!(r.out_digest, sha256(g.out), "{name}/{via}: out_digest = sha256(out)");
    assert_eq!(
        artifact_digest(r),
        hex32(g.artifact_digest),
        "{name}/{via}: artifact_digest"
    );
}

#[test]
fn empty_out_digest_constant() {
    assert_eq!(sha256(b""), hex32(EMPTY_SHA256));
}

#[test]
fn parity_halt() {
    let code = [0x02, 0x4f, 0x09, 0x02, 0x4b, 0x09, 0x02, 0x0a, 0x09, 0x00];
    check(
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
fn parity_budget_exceed() {
    check(
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
fn parity_host_escape_e0() {
    check(
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
    // Side condition (not hashed): mem[0] stays 0xE0.
    let mut vm = Vm::new(&[0xe0], 0, ResourceBudget::new(8, 0, 1), Cap::IoOutBounded).unwrap();
    vm.run();
    assert_eq!(vm.mem[0], 0xE0);
}

const OUT_FILL_CODE: [u8; 10] = [0x02, 0x58, 0x09, 0x02, 0x58, 0x09, 0x02, 0x58, 0x09, 0x00];

#[test]
fn parity_out_fill_ok() {
    check(
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
fn parity_out_fill_overflow() {
    check(
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
fn parity_mem_oob() {
    check(
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
