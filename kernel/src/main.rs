#![no_main]
#![no_std]

extern crate alloc;

mod cli;
mod serial;

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::chamber::{chamber_ratify, demo_policy_pass_spec, ChamberDecision};
use bilkrnl_core::digest::hex32;
use bilkrnl_core::enclave::{blast, EnclaveSlot};
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::fixture_policy_pass_program;
use uefi::prelude::*;
use uefi::runtime::{self, ResetType};

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    serial::write_str("bilkrnl: ready\r\n");

    // Auto demo: sealed policy gate (PASS) → dispose → ratify (TC-01).
    let mut slot = EnclaveSlot::new();
    let (entry, code) = fixture_policy_pass_program();
    let img = BytecodeImage::new(
        entry,
        code,
        Cap::IoOutBounded,
        ResourceBudget::new(128, 8, 1),
    )
    .expect("demo image");

    match blast(&mut slot, img) {
        Ok(result) => {
            serial::write_str("blast: done\r\n");
            match chamber_ratify(&result, &demo_policy_pass_spec()) {
                ChamberDecision::Ratified { artifact_digest } => {
                    serial::write_str("chamber: RATIFIED ");
                    serial::write_str(hex32(&artifact_digest).as_str());
                    serial::write_str("\r\n");
                }
                ChamberDecision::Reject { .. } => {
                    serial::write_str("chamber: REJECT\r\n");
                }
            }
        }
        Err(_) => serial::write_str("blast: ERROR\r\n"),
    }

    // Serial CLI surface (help + blast parity commands).
    cli::run_demo_commands();

    serial::write_str("bilkrnl: halt\r\n");
    // Shut down QEMU virt via UEFI reset.
    runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None);
}
