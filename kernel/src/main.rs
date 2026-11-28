#![no_main]
#![no_std]

extern crate alloc;

mod cli;
mod serial;
mod softcpu_mmio;

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::chamber::{
    chamber_ratify, demo_policy_pass_spec, demo_rev4_pass_spec, ChamberDecision,
};
use bilkrnl_core::digest::hex32;
use bilkrnl_core::enclave::{blast, EnclaveSlot};
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::{fixture_policy_pass_program, fixture_rev4_pass_program, EnclaveResult};
use uefi::prelude::*;
use uefi::runtime::{self, ResetType};

fn blast_auto(img: BytecodeImage) -> Result<(EnclaveResult, &'static str), ()> {
    if let Some(r) = softcpu_mmio::try_blast(&img) {
        return Ok((r, "softcpu"));
    }
    let mut slot = EnclaveSlot::new();
    blast(&mut slot, img).map(|r| (r, "interp")).map_err(|_| ())
}

fn ratify_line(label: &str, _result: &EnclaveResult, decision: ChamberDecision) {
    match decision {
        ChamberDecision::Ratified { artifact_digest } => {
            serial::write_str(label);
            serial::write_str(" RATIFIED ");
            serial::write_str(hex32(&artifact_digest).as_str());
            serial::write_str("\r\n");
        }
        ChamberDecision::Reject { .. } => {
            serial::write_str(label);
            serial::write_str(" REJECT\r\n");
        }
    }
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    serial::write_str("bilkrnl: ready\r\n");

    // Backend probe (softcpu MMIO vs interpreter).
    let backend = if softcpu_mmio::probe_mmio_base().is_some() {
        serial::write_str("softcpu: probed\r\n");
        "softcpu"
    } else {
        serial::write_str("softcpu: absent\r\n");
        "interp"
    };
    let _ = backend;

    // Workload 1: sealed policy gate (PASS) → dispose → ratify (TC-01 / TC-16).
    {
        let (entry, code) = fixture_policy_pass_program();
        let img = BytecodeImage::new(
            entry,
            code,
            Cap::IoOutBounded,
            ResourceBudget::new(128, 8, 1),
        )
        .expect("policy image");
        match blast_auto(img) {
            Ok((result, via)) => {
                serial::write_str("blast: policy via ");
                serial::write_str(via);
                serial::write_str("\r\n");
                ratify_line(
                    "chamber:",
                    &result,
                    chamber_ratify(&result, &demo_policy_pass_spec()),
                );
            }
            Err(()) => serial::write_str("blast: ERROR\r\n"),
        }
    }

    // Workload 2: sealed attested reverse (BIL! → !LIB).
    {
        let (entry, code) = fixture_rev4_pass_program();
        let img = BytecodeImage::new(
            entry,
            code,
            Cap::IoOutBounded,
            ResourceBudget::new(64, 8, 1),
        )
        .expect("rev4 image");
        match blast_auto(img) {
            Ok((result, via)) => {
                serial::write_str("blast: rev4 via ");
                serial::write_str(via);
                serial::write_str("\r\n");
                ratify_line(
                    "chamber:rev4:",
                    &result,
                    chamber_ratify(&result, &demo_rev4_pass_spec()),
                );
            }
            Err(()) => serial::write_str("blast:rev4: ERROR\r\n"),
        }
    }

    // Serial CLI surface (help + blast parity commands) — interpreter path for parity.
    cli::run_demo_commands();

    serial::write_str("bilkrnl: halt\r\n");
    runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None);
}
