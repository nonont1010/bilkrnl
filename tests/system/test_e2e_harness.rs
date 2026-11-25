//! Host-side e2e of load→run→dispose→ratify (mirrors TC-01 pipeline).

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::chamber::{chamber_ratify, demo_ok_spec, ChamberDecision};
use bilkrnl_core::enclave::{blast, EnclaveSlot};
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::fixture_ok_program;

#[test]
fn tc01_blast_chamber_pipeline() {
    let mut slot = EnclaveSlot::new();
    let img = BytecodeImage::new(
        0,
        fixture_ok_program(),
        Cap::IoOutBounded,
        ResourceBudget::new(64, 8, 1),
    )
    .unwrap();
    let result = blast(&mut slot, img).unwrap();
    assert_eq!(result.out, b"OK\n");
    match chamber_ratify(&result, &demo_ok_spec()) {
        ChamberDecision::Ratified { artifact_digest } => {
            assert_ne!(artifact_digest, [0u8; 32]);
        }
        ChamberDecision::Reject { reason } => panic!("unexpected reject: {:?}", reason),
    }
    let mem = slot.disposed_mem(bilkrnl_core::enclave::EnclaveId(1)).unwrap();
    assert!(mem.iter().all(|&b| b == 0));
}
