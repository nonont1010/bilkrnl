//! Host-side e2e of load→run→dispose→ratify (mirrors TC-01 pipeline).

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::chamber::{
    chamber_ratify, demo_policy_fail_spec, demo_policy_pass_spec, demo_rev4_fail_spec,
    demo_rev4_pass_spec, demo_rot4_fail_spec, demo_rot4_pass_spec, ChamberDecision,
    ChamberReject,
};
use bilkrnl_core::digest::hex32;
use bilkrnl_core::enclave::{artifact_digest, blast, EnclaveSlot};
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::{
    fixture_policy_fail_program, fixture_policy_pass_program, fixture_rev4_fail_program,
    fixture_rev4_pass_program, fixture_rot4_fail_program, fixture_rot4_pass_program,
};

// Keep in lockstep with tests/system/golden/policy_pass.artifact_digest (TC-16 host gate).
const POLICY_PASS_ARTIFACT: &str =
    "14ad56f22141c89095b78d1eb28cd846b3ad2c331a5c693f24a9a68711b317ea";

#[test]
fn tc01_blast_chamber_pipeline() {
    let mut slot = EnclaveSlot::new();
    let (entry, code) = fixture_policy_pass_program();
    let img = BytecodeImage::new(
        entry,
        code,
        Cap::IoOutBounded,
        ResourceBudget::new(128, 8, 1),
    )
    .unwrap();
    let result = blast(&mut slot, img).unwrap();
    assert_eq!(result.out, b"PASS\n");
    assert_eq!(result.caps_at_exit, Cap::Compute);
    assert_eq!(result.cycles, 15);
    match chamber_ratify(&result, &demo_policy_pass_spec()) {
        ChamberDecision::Ratified { artifact_digest: d } => {
            assert_eq!(hex32(&d).as_str(), POLICY_PASS_ARTIFACT);
            assert_eq!(hex32(&artifact_digest(&result)).as_str(), POLICY_PASS_ARTIFACT);
        }
        ChamberDecision::Reject { reason } => panic!("unexpected reject: {:?}", reason),
    }
    let mem = slot.disposed_mem(bilkrnl_core::enclave::EnclaveId(1)).unwrap();
    assert!(mem.iter().all(|&b| b == 0));
}

#[test]
fn policy_fail_ratifies_fail_spec() {
    let mut slot = EnclaveSlot::new();
    let (entry, code) = fixture_policy_fail_program();
    let img = BytecodeImage::new(
        entry,
        code,
        Cap::IoOutBounded,
        ResourceBudget::new(128, 8, 1),
    )
    .unwrap();
    let result = blast(&mut slot, img).unwrap();
    assert_eq!(result.out, b"FAIL\n");
    assert_eq!(result.cycles, 21);
    assert!(matches!(
        chamber_ratify(&result, &demo_policy_fail_spec()),
        ChamberDecision::Ratified { .. }
    ));
    // PASS spec must reject FAIL out.
    match chamber_ratify(&result, &demo_policy_pass_spec()) {
        ChamberDecision::Reject {
            reason: ChamberReject::OutPredicateFailed,
        } => {}
        other => panic!("{:?}", other),
    }
}

#[test]
fn rev4_pass_ratifies_and_pins_digests() {
    let mut slot = EnclaveSlot::new();
    let (entry, code) = fixture_rev4_pass_program();
    let img = BytecodeImage::new(
        entry,
        code,
        Cap::IoOutBounded,
        ResourceBudget::new(64, 8, 1),
    )
    .unwrap();
    let result = blast(&mut slot, img).unwrap();
    assert_eq!(result.out, b"!LIB");
    assert_eq!(result.caps_at_exit, Cap::Compute);
    let d = match chamber_ratify(&result, &demo_rev4_pass_spec()) {
        ChamberDecision::Ratified { artifact_digest: d } => d,
        ChamberDecision::Reject { reason } => panic!("unexpected reject: {:?}", reason),
    };
    // Pin after first green run (also recorded in softcpu/PARITY.md + golden file).
    let art = hex32(&artifact_digest(&result));
    assert_eq!(hex32(&d).as_str(), art.as_str());
    assert_eq!(
        art.as_str(),
        include_str!("golden/rev4_pass.artifact_digest").trim()
    );
}

#[test]
fn rev4_fail_ratifies_fail_spec_only() {
    let mut slot = EnclaveSlot::new();
    let (entry, code) = fixture_rev4_fail_program();
    let img = BytecodeImage::new(
        entry,
        code,
        Cap::IoOutBounded,
        ResourceBudget::new(64, 8, 1),
    )
    .unwrap();
    let result = blast(&mut slot, img).unwrap();
    assert_eq!(result.out, b"XXXX");
    assert!(matches!(
        chamber_ratify(&result, &demo_rev4_fail_spec()),
        ChamberDecision::Ratified { .. }
    ));
    match chamber_ratify(&result, &demo_rev4_pass_spec()) {
        ChamberDecision::Reject {
            reason: ChamberReject::OutPredicateFailed,
        } => {}
        other => panic!("{:?}", other),
    }
}

#[test]
fn rot4_pass_ratifies_and_pins_digests() {
    let mut slot = EnclaveSlot::new();
    let (entry, code) = fixture_rot4_pass_program();
    let img = BytecodeImage::new(
        entry,
        code,
        Cap::IoOutBounded,
        ResourceBudget::new(64, 8, 1),
    )
    .unwrap();
    let result = blast(&mut slot, img).unwrap();
    assert_eq!(result.out, b"IL1B");
    assert_eq!(result.caps_at_exit, Cap::Compute);
    let d = match chamber_ratify(&result, &demo_rot4_pass_spec()) {
        ChamberDecision::Ratified { artifact_digest: d } => d,
        ChamberDecision::Reject { reason } => panic!("unexpected reject: {:?}", reason),
    };
    let art = hex32(&artifact_digest(&result));
    assert_eq!(hex32(&d).as_str(), art.as_str());
    assert_eq!(
        art.as_str(),
        include_str!("golden/rot4_pass.artifact_digest").trim()
    );
}

#[test]
fn rot4_fail_ratifies_fail_spec_only() {
    let mut slot = EnclaveSlot::new();
    let (entry, code) = fixture_rot4_fail_program();
    let img = BytecodeImage::new(
        entry,
        code,
        Cap::IoOutBounded,
        ResourceBudget::new(64, 8, 1),
    )
    .unwrap();
    let result = blast(&mut slot, img).unwrap();
    assert_eq!(result.out, b"XXXX");
    assert!(matches!(
        chamber_ratify(&result, &demo_rot4_fail_spec()),
        ChamberDecision::Ratified { .. }
    ));
    match chamber_ratify(&result, &demo_rot4_pass_spec()) {
        ChamberDecision::Reject {
            reason: ChamberReject::OutPredicateFailed,
        } => {}
        other => panic!("{:?}", other),
    }
}
