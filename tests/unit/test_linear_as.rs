//! TC-05: linear address space page boundaries.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::isa::{ExitReason, FaultKind, Op, PAGE_SIZE, Vm};

#[test]
fn tc05_one_page_store_ok() {
    // STORE addr 0 with 1 page
    let code = vec![Op::LoadImm as u8, 0xAB, Op::Store as u8, 0x00, 0x00, Op::Halt as u8];
    let mut vm = Vm::new(&code, 0, ResourceBudget::new(16, 0, 1), Cap::Compute).unwrap();
    let r = vm.run();
    assert_eq!(r.exit, ExitReason::Halt);
    assert!(r.mem_high_water >= 1);
}

#[test]
fn tc05_store_oob_fault() {
    let addr = (PAGE_SIZE as u16).to_le_bytes(); // page 1 when only 1 page mapped
    let code = vec![
        Op::LoadImm as u8,
        0xAB,
        Op::Store as u8,
        addr[0],
        addr[1],
        Op::Halt as u8,
    ];
    let mut vm = Vm::new(&code, 0, ResourceBudget::new(16, 0, 1), Cap::Compute).unwrap();
    let r = vm.run();
    assert_eq!(r.exit, ExitReason::Fault(FaultKind::MemOob));
}

#[test]
fn tc05_zero_pages_rejected() {
    assert!(Vm::new(&[Op::Halt as u8], 0, ResourceBudget::new(1, 0, 0), Cap::Compute).is_err());
}
