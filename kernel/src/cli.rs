//! Serial CLI surface — thin adapter over core commands.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::chamber::{chamber_ratify, demo_policy_pass_spec, ChamberDecision};
use bilkrnl_core::digest::hex32;
use bilkrnl_core::enclave::{blast, EnclaveSlot};
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::fixture_policy_pass_program;

use crate::serial;

/// V1: emit help + one-shot blast via the same core path (parity with harness).
pub fn run_demo_commands() {
    serial::write_str("cli: help enclave-load enclave-run enclave-dispose chamber-ratify blast\r\n");
    serial::write_str("cli: blast\r\n");

    let mut slot = EnclaveSlot::new();
    let (entry, code) = fixture_policy_pass_program();
    let img = BytecodeImage::new(
        entry,
        code,
        Cap::IoOutBounded,
        ResourceBudget::new(128, 8, 1),
    )
    .expect("cli image");

    match blast(&mut slot, img) {
        Ok(result) => match chamber_ratify(&result, &demo_policy_pass_spec()) {
            ChamberDecision::Ratified { artifact_digest } => {
                serial::write_str("cli: RATIFIED ");
                serial::write_str(hex32(&artifact_digest).as_str());
                serial::write_str("\r\n");
            }
            ChamberDecision::Reject { .. } => serial::write_str("cli: REJECT\r\n"),
        },
        Err(_) => serial::write_str("cli: ERROR\r\n"),
    }
}
