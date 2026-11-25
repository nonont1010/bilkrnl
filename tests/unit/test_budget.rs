//! TC-03: cycle budget boundaries.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::isa::{ExitReason, Op, Vm};

fn run_nops(max_cycles: u64, nops: usize) -> ExitReason {
    let mut code = vec![Op::Nop as u8; nops];
    code.push(Op::Halt as u8);
    let mut vm = Vm::new(
        &code,
        0,
        ResourceBudget::new(max_cycles, 0, 1),
        Cap::Compute,
    )
    .unwrap();
    vm.run().exit
}

#[test]
fn tc03_halt_at_max_cycles_minus_one() {
    // nops=2 + halt = 3 cycles; budget 3 → Halt
    assert_eq!(run_nops(3, 2), ExitReason::Halt);
}

#[test]
fn tc03_halt_exactly_at_max() {
    assert_eq!(run_nops(1, 0), ExitReason::Halt);
}

#[test]
fn tc03_budget_exceeded_when_over() {
    // 2 nops + halt needs 3 cycles; budget 2 → BudgetExceeded during 3rd charge
    assert_eq!(run_nops(2, 2), ExitReason::BudgetExceeded);
}
