//! Enclave load / run / dispose — single-flight V1.

use alloc::vec::Vec;

use crate::digest::sha256;
use crate::image::{BytecodeImage, ImageError};
use crate::isa::{EnclaveResult, Vm};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EnclaveId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    Image(ImageError),
    Busy(EnclaveId),
    VmInit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    NotFound(EnclaveId),
    Disposed(EnclaveId),
    AlreadyRan(EnclaveId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisposeError {
    NotFound(EnclaveId),
}

enum Phase {
    Loaded(Vm),
    Ran {
        #[allow(dead_code)]
        result: EnclaveResult,
        mem: Vec<u8>,
    },
    Disposed { zero_probe: Vec<u8> },
}

/// Single-enclave slot (V1 concurrency model).
pub struct EnclaveSlot {
    next_id: u64,
    active: Option<(EnclaveId, Phase)>,
}

impl Default for EnclaveSlot {
    fn default() -> Self {
        Self::new()
    }
}

impl EnclaveSlot {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            active: None,
        }
    }

    pub fn load(&mut self, img: BytecodeImage) -> Result<EnclaveId, LoadError> {
        if let Some((id, phase)) = &self.active {
            if !matches!(phase, Phase::Disposed { .. }) {
                return Err(LoadError::Busy(*id));
            }
        }
        let vm = Vm::new(&img.code, img.entry, img.budget, img.initial_caps)
            .map_err(|_| LoadError::VmInit)?;
        let id = EnclaveId(self.next_id);
        self.next_id += 1;
        self.active = Some((id, Phase::Loaded(vm)));
        Ok(id)
    }

    pub fn load_bytes(&mut self, bytes: &[u8]) -> Result<EnclaveId, LoadError> {
        let img = BytecodeImage::decode(bytes).map_err(LoadError::Image)?;
        self.load(img)
    }

    pub fn run(&mut self, id: EnclaveId) -> Result<EnclaveResult, RunError> {
        {
            let (slot_id, phase) = self.active.as_ref().ok_or(RunError::NotFound(id))?;
            if *slot_id != id {
                return Err(RunError::NotFound(id));
            }
            match phase {
                Phase::Disposed { .. } => return Err(RunError::Disposed(id)),
                Phase::Ran { .. } => return Err(RunError::AlreadyRan(id)),
                Phase::Loaded(_) => {}
            }
        }
        let (_, phase) = self.active.take().unwrap();
        let mut vm = match phase {
            Phase::Loaded(vm) => vm,
            _ => unreachable!(),
        };
        let result = vm.run();
        let mem = vm.mem.clone();
        self.active = Some((
            id,
            Phase::Ran {
                result: result.clone(),
                mem,
            },
        ));
        Ok(result)
    }

    pub fn dispose(&mut self, id: EnclaveId) -> Result<(), DisposeError> {
        let (slot_id, phase) = self.active.as_mut().ok_or(DisposeError::NotFound(id))?;
        if *slot_id != id {
            return Err(DisposeError::NotFound(id));
        }
        match phase {
            Phase::Disposed { .. } => Ok(()), // idempotent
            Phase::Loaded(vm) => {
                vm.zeroize();
                let zero_probe = vm.mem.clone();
                *phase = Phase::Disposed { zero_probe };
                Ok(())
            }
            Phase::Ran { mem, .. } => {
                for b in mem.iter_mut() {
                    *b = 0;
                }
                let zero_probe = mem.clone();
                *phase = Phase::Disposed { zero_probe };
                Ok(())
            }
        }
    }

    /// Probe disposed memory (integration/zeroization tests).
    pub fn disposed_mem(&self, id: EnclaveId) -> Option<&[u8]> {
        match &self.active {
            Some((sid, Phase::Disposed { zero_probe })) if *sid == id => Some(zero_probe.as_slice()),
            _ => None,
        }
    }

    pub fn clear_disposed(&mut self) {
        if matches!(self.active, Some((_, Phase::Disposed { .. }))) {
            self.active = None;
        }
    }
}

pub fn enclave_load(slot: &mut EnclaveSlot, img: BytecodeImage) -> Result<EnclaveId, LoadError> {
    slot.load(img)
}

pub fn enclave_run(slot: &mut EnclaveSlot, id: EnclaveId) -> Result<EnclaveResult, RunError> {
    slot.run(id)
}

pub fn enclave_dispose(slot: &mut EnclaveSlot, id: EnclaveId) -> Result<(), DisposeError> {
    slot.dispose(id)
}

/// One-shot blast pipeline: load → run → dispose → return result (caller ratifies).
pub fn blast(slot: &mut EnclaveSlot, img: BytecodeImage) -> Result<EnclaveResult, BlastError> {
    let id = slot.load(img).map_err(BlastError::Load)?;
    let result = slot.run(id).map_err(BlastError::Run)?;
    slot.dispose(id).map_err(BlastError::Dispose)?;
    Ok(result)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlastError {
    Load(LoadError),
    Run(RunError),
    Dispose(DisposeError),
}

/// Artifact digest over exit tag + out bytes + digests (stable ratification input).
pub fn artifact_bytes(result: &EnclaveResult) -> Vec<u8> {
    let mut v = Vec::new();
    let tag: u8 = match result.exit {
        crate::isa::ExitReason::Halt => 0,
        crate::isa::ExitReason::BudgetExceeded => 1,
        crate::isa::ExitReason::Fault(_) => 2,
    };
    v.push(tag);
    v.extend_from_slice(&result.cycles.to_le_bytes());
    v.extend_from_slice(&(result.out.len() as u32).to_le_bytes());
    v.extend_from_slice(&result.out);
    v.extend_from_slice(&result.code_digest);
    v.extend_from_slice(&result.out_digest);
    v
}

pub fn artifact_digest(result: &EnclaveResult) -> [u8; 32] {
    sha256(&artifact_bytes(result))
}
