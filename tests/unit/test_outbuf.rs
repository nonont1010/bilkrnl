//! TC-04: out buffer 0 / max / max+1.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::isa::{ExitReason, FaultKind, Op, Vm};

fn outs(n: usize, max_out: u32) -> bilkrnl_core::isa::EnclaveResult {
    let mut code = Vec::new();
    for _ in 0..n {
        code.push(Op::LoadImm as u8);
        code.push(b'X');
        code.push(Op::Out as u8);
    }
    code.push(Op::Halt as u8);
    let mut vm = Vm::new(
        &code,
        0,
        ResourceBudget::new(10_000, max_out, 1),
        Cap::IoOutBounded,
    )
    .unwrap();
    vm.run()
}

#[test]
fn tc04_zero_out_ok() {
    let r = outs(0, 0);
    assert_eq!(r.exit, ExitReason::Halt);
    assert!(r.out.is_empty());
}

#[test]
fn tc04_max_out_ok() {
    let r = outs(3, 3);
    assert_eq!(r.exit, ExitReason::Halt);
    assert_eq!(r.out, b"XXX");
}

#[test]
fn tc04_max_plus_one_fault() {
    let r = outs(3, 2);
    assert_eq!(r.exit, ExitReason::Fault(FaultKind::OutOverflow));
}
