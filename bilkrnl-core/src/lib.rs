//! bilkrnl-core: capability ratchet, soft ISA, enclave lifecycle, blast chamber.
//!
//! `no_std` + `alloc` so the same contracts run in the UEFI kernel and host tests.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod budget;
pub mod cap;
pub mod chamber;
pub mod digest;
pub mod enclave;
pub mod image;
pub mod isa;

pub use budget::ResourceBudget;
pub use cap::{Cap, CapError, CapPoset};
pub use chamber::{
    chamber_ratify, demo_ok_spec, demo_policy_fail_spec, demo_policy_pass_spec,
    demo_rev4_fail_spec, demo_rev4_pass_spec, ChamberDecision, ChamberReject, OutPred, RatifySpec,
};
pub use enclave::{
    enclave_dispose, enclave_load, enclave_run, blast, DisposeError, EnclaveId, EnclaveSlot,
    LoadError, RunError,
};
pub use image::{BytecodeImage, ImageError, IMAGE_MAGIC};
pub use isa::{EnclaveResult, ExitReason, FaultKind, Vm, PAGE_SIZE};
