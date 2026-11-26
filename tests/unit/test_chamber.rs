//! TC-13: chamber reject on wrong digest / exit.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::chamber::{
    chamber_ratify, ChamberDecision, ChamberReject, OutPred, RatifySpec,
};
use bilkrnl_core::enclave::{blast, EnclaveSlot};
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::{fixture_ok_program, ExitReason};

fn ok_result() -> bilkrnl_core::isa::EnclaveResult {
    let mut slot = EnclaveSlot::new();
    let img = BytecodeImage::new(
        0,
        fixture_ok_program(),
        Cap::IoOutBounded,
        ResourceBudget::new(64, 8, 1),
    )
    .unwrap();
    blast(&mut slot, img).unwrap()
}

#[test]
fn tc13_ratify_ok() {
    let r = ok_result();
    let spec = RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 64,
        max_out_bytes: 8,
        required_caps_ceiling: Cap::IoOutBounded,
        out_predicate: OutPred::ExactBytes(b"OK\n"),
    };
    assert!(matches!(
        chamber_ratify(&r, &spec),
        ChamberDecision::Ratified { .. }
    ));
}

#[test]
fn tc13_wrong_out_reject() {
    let r = ok_result();
    let spec = RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 64,
        max_out_bytes: 8,
        required_caps_ceiling: Cap::IoOutBounded,
        out_predicate: OutPred::ExactBytes(b"NO"),
    };
    match chamber_ratify(&r, &spec) {
        ChamberDecision::Reject {
            reason: ChamberReject::OutPredicateFailed,
        } => {}
        other => panic!("{:?}", other),
    }
}

#[test]
fn tc13_wrong_exit_reject() {
    let mut slot = EnclaveSlot::new();
    let img = BytecodeImage::new(
        0,
        vec![0x7F],
        Cap::Compute,
        ResourceBudget::new(8, 0, 1),
    )
    .unwrap();
    let r = blast(&mut slot, img).unwrap();
    let spec = RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 64,
        max_out_bytes: 8,
        required_caps_ceiling: Cap::IoOutBounded,
        out_predicate: OutPred::Any,
    };
    match chamber_ratify(&r, &spec) {
        ChamberDecision::Reject {
            reason: ChamberReject::ExitNotAllowed,
        } => {}
        other => panic!("{:?}", other),
    }
}

#[test]
fn tc13_cycles_over_reject() {
    let r = ok_result();
    let spec = RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 1,
        max_out_bytes: 8,
        required_caps_ceiling: Cap::IoOutBounded,
        out_predicate: OutPred::Any,
    };
    assert!(matches!(
        chamber_ratify(&r, &spec),
        ChamberDecision::Reject {
            reason: ChamberReject::CyclesOverBudget { .. }
        }
    ));
}

#[test]
fn tc13_out_too_large_reject() {
    let r = ok_result();
    assert!(r.out.len() > 1);
    let spec = RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 1_000_000,
        max_out_bytes: 1,
        required_caps_ceiling: Cap::IoOutBounded,
        out_predicate: OutPred::Any,
    };
    match chamber_ratify(&r, &spec) {
        ChamberDecision::Reject {
            reason: ChamberReject::OutTooLarge { len, max },
        } => {
            assert_eq!(len as usize, r.out.len());
            assert_eq!(max, 1);
        }
        other => panic!("{:?}", other),
    }
}

#[test]
fn tc13_cap_ceiling_reject() {
    let r = ok_result();
    assert_eq!(r.caps_at_exit, Cap::IoOutBounded);
    let spec = RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 1_000_000,
        max_out_bytes: 1024,
        required_caps_ceiling: Cap::Compute,
        out_predicate: OutPred::Any,
    };
    match chamber_ratify(&r, &spec) {
        ChamberDecision::Reject {
            reason: ChamberReject::CapCeilingViolated { at_exit, ceiling },
        } => {
            assert_eq!(at_exit, Cap::IoOutBounded);
            assert_eq!(ceiling, Cap::Compute);
        }
        other => panic!("{:?}", other),
    }
}

#[test]
fn policy_pass_spec_ratifies() {
    use bilkrnl_core::chamber::demo_policy_pass_spec;
    use bilkrnl_core::isa::fixture_policy_pass_program;

    let mut slot = EnclaveSlot::new();
    let (entry, code) = fixture_policy_pass_program();
    let img = BytecodeImage::new(
        entry,
        code,
        Cap::IoOutBounded,
        ResourceBudget::new(128, 8, 1),
    )
    .unwrap();
    let r = blast(&mut slot, img).unwrap();
    assert!(matches!(
        chamber_ratify(&r, &demo_policy_pass_spec()),
        ChamberDecision::Ratified { .. }
    ));
}
