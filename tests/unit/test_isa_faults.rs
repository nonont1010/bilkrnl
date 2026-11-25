//! TC-07: illegal opcode / jump OOB.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::isa::{ExitReason, FaultKind, Op, Vm};

#[test]
fn tc07_illegal_opcode() {
    let mut vm = Vm::new(&[0x7F], 0, ResourceBudget::new(8, 0, 1), Cap::Compute).unwrap();
    assert_eq!(
        vm.run().exit,
        ExitReason::Fault(FaultKind::IllegalOpcode(0x7F))
    );
}

#[test]
fn tc07_jump_oob() {
    let addr = (4096u16).to_le_bytes();
    let code = vec![Op::Jmp as u8, addr[0], addr[1], Op::Halt as u8];
    let mut vm = Vm::new(&code, 0, ResourceBudget::new(8, 0, 1), Cap::Compute).unwrap();
    assert_eq!(vm.run().exit, ExitReason::Fault(FaultKind::PcOob));
}
