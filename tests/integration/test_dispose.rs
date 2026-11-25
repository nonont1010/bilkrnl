//! TC-10: dispose twice idempotent; run after dispose errors.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::enclave::{EnclaveSlot, RunError};
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::Op;

#[test]
fn tc10_dispose_twice_ok() {
    let mut slot = EnclaveSlot::new();
    let img = BytecodeImage::new(
        0,
        vec![Op::Halt as u8],
        Cap::Compute,
        ResourceBudget::new(8, 0, 1),
    )
    .unwrap();
    let id = slot.load(img).unwrap();
    slot.run(id).unwrap();
    slot.dispose(id).unwrap();
    slot.dispose(id).unwrap();
}

#[test]
fn tc10_run_after_dispose_errors() {
    let mut slot = EnclaveSlot::new();
    let img = BytecodeImage::new(
        0,
        vec![Op::Halt as u8],
        Cap::Compute,
        ResourceBudget::new(8, 0, 1),
    )
    .unwrap();
    let id = slot.load(img).unwrap();
    slot.dispose(id).unwrap();
    match slot.run(id) {
        Err(RunError::Disposed(d)) => assert_eq!(d, id),
        other => panic!("{:?}", other),
    }
}
