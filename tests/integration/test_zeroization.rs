//! TC-14: enclave RAM reads as zero after dispose.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::enclave::EnclaveSlot;
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::{fixture_ok_program, Op};

#[test]
fn tc14_memory_zero_after_dispose() {
    let mut slot = EnclaveSlot::new();
    let img = BytecodeImage::new(
        0,
        fixture_ok_program(),
        Cap::IoOutBounded,
        ResourceBudget::new(64, 8, 1),
    )
    .unwrap();
    let id = slot.load(img).unwrap();
    let _ = slot.run(id).unwrap();
    slot.dispose(id).unwrap();
    let mem = slot.disposed_mem(id).expect("disposed mem");
    assert!(mem.iter().all(|&b| b == 0), "non-zero residual after dispose");
}

#[test]
fn tc14_dispose_before_run_zeros_code() {
    let mut slot = EnclaveSlot::new();
    let img = BytecodeImage::new(
        0,
        vec![Op::LoadImm as u8, 0xFF, Op::Halt as u8],
        Cap::Compute,
        ResourceBudget::new(8, 0, 1),
    )
    .unwrap();
    let id = slot.load(img).unwrap();
    slot.dispose(id).unwrap();
    let mem = slot.disposed_mem(id).unwrap();
    assert!(mem.iter().all(|&b| b == 0));
}
