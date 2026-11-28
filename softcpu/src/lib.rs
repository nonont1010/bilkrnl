//! Host MMIO soft-CPU simulator for Milestone 1C.
//!
//! # Honest SoftISA body
//!
//! After `COMMIT_LOAD`, this crate calls [`bilkrnl_core::isa::Vm`] for `STEP` /
//! `RUN`. Digests therefore match the 1A interpreter by construction until a
//! future SoftISA fork. The value of this crate is the **MMIO protocol seam**
//! (register map, load/run/dispose lifecycle, `EnclaveResult` rebuild) — not a
//! second opcode implementation.
//!
//! QEMU `-device bil-softcpu` and FPGA/RTL backends remain deferred.

pub mod device;
pub mod regs;
pub mod result;

pub use device::{pack_fault, unpack_fault, SoftCpu};
pub use regs::*;
pub use result::{artifact_digest_from_mmio, blast_via_mmio, enclave_result_from_mmio};
