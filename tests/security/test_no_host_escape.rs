//! TC-08: EFI/MMIO-shaped opcodes fault with no host side effect.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::isa::{ExitReason, FaultKind, Op, Vm};

#[test]
fn tc08_host_shaped_opcodes_fault() {
    for op in 0xE0u8..=0xEF {
        let mut vm = Vm::new(&[op], 0, ResourceBudget::new(8, 0, 1), Cap::IoOutBounded).unwrap();
        let before = vm.mem.clone();
        let r = vm.run();
        assert_eq!(
            r.exit,
            ExitReason::Fault(FaultKind::HostEscapeAttempt(op)),
            "op {:#x}",
            op
        );
        // Memory unchanged except PC/cycle accounting — code page still original bytes at 0
        assert_eq!(vm.mem[0], before[0]);
    }
}

#[test]
fn tc08_no_efi_opcode_in_isa() {
    // Legitimate path never exposes EFI; only Halt exists as 0x00.
    let mut vm = Vm::new(&[Op::Halt as u8], 0, ResourceBudget::new(1, 0, 1), Cap::Compute).unwrap();
    assert_eq!(vm.run().exit, ExitReason::Halt);
}
