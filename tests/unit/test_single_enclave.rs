//! TC-11: single-flight load — second load rejected while busy.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::enclave::{EnclaveSlot, LoadError};
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::{fixture_ok_program, Op};

fn img() -> BytecodeImage {
    BytecodeImage::new(
        0,
        vec![Op::Halt as u8],
        Cap::Compute,
        ResourceBudget::new(8, 0, 1),
    )
    .unwrap()
}

#[test]
fn tc11_second_load_busy() {
    let mut slot = EnclaveSlot::new();
    let id = slot.load(img()).unwrap();
    match slot.load(img()) {
        Err(LoadError::Busy(b)) => assert_eq!(b, id),
        other => panic!("expected Busy, got {:?}", other),
    }
}

#[test]
fn tc11_load_after_dispose_ok() {
    let mut slot = EnclaveSlot::new();
    let id = slot.load(img()).unwrap();
    slot.dispose(id).unwrap();
    assert!(slot.load(BytecodeImage::new(
        0,
        fixture_ok_program(),
        Cap::IoOutBounded,
        ResourceBudget::new(64, 8, 1),
    )
    .unwrap())
    .is_ok());
}
