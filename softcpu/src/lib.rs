//! Host MMIO **protocol** simulator for Milestone 1C.
//!
//! # Not a SoftISA body
//!
//! After `COMMIT_LOAD`, this crate calls [`bilkrnl_core::isa::Vm`] for `STEP` /
//! `RUN`. Digests therefore match the 1A interpreter **by construction**. The
//! value of this crate is the **MMIO register protocol** (load/run/dispose
//! lifecycle, `EnclaveResult` rebuild) — not a second opcode implementation.
//!
//! # Where the 1C SoftISA body lives
//!
//! Independent SoftISA execution is the C port in `softcpu/qemu/bil_isa.c`,
//! exercised by `make -C softcpu/qemu test-isa` and by QEMU `-device bil-softcpu`
//! (`scripts/build-qemu-bil-softcpu.sh`, `SOFTCPU=1`). FPGA/RTL remain deferred.

pub mod device;
pub mod regs;
pub mod result;

pub use device::{pack_fault, unpack_fault, SoftCpu};
pub use regs::*;
pub use result::{artifact_digest_from_mmio, blast_via_mmio, enclave_result_from_mmio};
