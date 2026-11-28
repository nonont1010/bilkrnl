//! B3.4 — 1A golden digests for bil1asm-lowered stack-bytecode programs.
//! Interpreter only (no 1C device, no `bil` binary).

use bil1asm::lower_stack_bc;
use bilkrnl_core::digest::sha256;
use bilkrnl_core::enclave::{artifact_digest, blast, EnclaveSlot};
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::{EnclaveResult, ExitReason, Vm};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Halt fixture digests (`softcpu/PARITY.md` §halt / `fixture_ok_program`).
const HALT_CODE: [u8; 10] = [0x02, 0x4f, 0x09, 0x02, 0x4b, 0x09, 0x02, 0x0a, 0x09, 0x00];
const HALT_CODE_DIGEST: &str = "0fe5a198c3048319789344d79fba856c45e178446346f1417fca0eed7cacf015";
const HALT_OUT_DIGEST: &str = "a12b7cb43c9d9134b5bb1b35e9096b66775d9e92e7611d1cc92b02edd6782a87";
const HALT_ARTIFACT_DIGEST: &str = "25351c265eba64629fc702c47cd44372035b3c53b6bf0a5b98f2e86cb1fe798d";

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
    code_bytes: &'static [u8],
    code_digest: &'static str,
    out_digest: &'static str,
    artifact_digest: &'static str,
}

fn check_lowered(name: &str, src: &str, g: &Golden) {
    let assembled = lower_stack_bc(src).unwrap_or_else(|e| panic!("{name}: lower: {e}"));
    let img = assembled.image;
    assert_eq!(img.code, g.code_bytes, "{name}: lowered code bytes");

    let mut vm = Vm::new(&img.code, img.entry, img.budget, img.initial_caps).unwrap();
    assert_digests(name, "Vm::run", &vm.run(), g);

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
fn lowered_ok_bc_txt_matches_halt_goldens() {
    let src = include_str!("../../tools/bil1asm/examples/ok.bc.txt");
    check_lowered(
        "ok.bc.txt",
        src,
        &Golden {
            exit: ExitReason::Halt,
            cycles: 7,
            out: b"OK\n",
            code_bytes: &HALT_CODE,
            code_digest: HALT_CODE_DIGEST,
            out_digest: HALT_OUT_DIGEST,
            artifact_digest: HALT_ARTIFACT_DIGEST,
        },
    );
}

#[test]
fn lowered_out_hello_bc_txt_matches_halt_goldens() {
    let src = include_str!("../../tools/bil1asm/examples/lower/out_hello.bc.txt");
    check_lowered(
        "out_hello.bc.txt",
        src,
        &Golden {
            exit: ExitReason::Halt,
            cycles: 7,
            out: b"OK\n",
            code_bytes: &HALT_CODE,
            code_digest: HALT_CODE_DIGEST,
            out_digest: HALT_OUT_DIGEST,
            artifact_digest: HALT_ARTIFACT_DIGEST,
        },
    );
}

#[test]
fn lowered_add_imm_slots_goldens() {
    let src = include_str!("../../tools/bil1asm/examples/lower/add_imm.bc.txt");
    check_lowered(
        "add_imm.bc.txt",
        src,
        &Golden {
            exit: ExitReason::Halt,
            cycles: 5,
            out: b"",
            code_bytes: &[
                0x02, 10, // ldi 10
                0x04, 11, 0, // st @11
                0x03, 11, 0, // ld @11
                0x05, 5, // addi 5
                0x00, // halt
            ],
            code_digest: "a98f13c3b49c2fb4b25262047c4b4505ed1269066896680315f628c96005bb3d",
            out_digest: EMPTY_SHA256,
            artifact_digest: "1dbe7975a1e666884e665f45d2346b3c66a6d4a7ebb6dc47dc618c26e8bdc560",
        },
    );
}
