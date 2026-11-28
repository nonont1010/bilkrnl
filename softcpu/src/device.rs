//! Host-side MMIO **protocol** simulator for `bil-softcpu`.
//!
//! After `COMMIT_LOAD`, execution uses `bilkrnl-core::isa::Vm` (silent wrap).
//! This module proves the register map and lifecycle, not an independent SoftISA.
//! The 1C SoftISA body is `softcpu/qemu/bil_isa.c` (QEMU `-device bil-softcpu`).
//! See `softcpu/README.md` and `docs/SOFTISA.md`.

use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::{ExitReason, FaultKind, Vm};

use crate::regs::*;

/// In-process MMIO aperture (64 KiB window semantics).
///
/// Name says “CPU” for historical reasons; the opcode engine is still [`Vm`].
pub struct SoftCpu {
    max_pages: u16,
    status: u32,
    load_err: u32,
    img_len: u32,
    staging: Vec<u8>,
    /// Present after successful `COMMIT_LOAD` until `RESET_ZEROIZE`.
    vm: Option<Vm>,
    /// Latched result window (populated by `RUN` / terminal `STEP`).
    exit_kind: u32,
    fault_detail: u32,
    cycles: u64,
    mem_high_water: u16,
    caps_at_exit: u8,
    out: Vec<u8>,
    out_read_idx: u32,
    code_digest: [u8; 32],
    out_digest: [u8; 32],
    zero_verified: u32,
}

impl SoftCpu {
    pub fn new(max_pages: u16) -> Self {
        let max_pages = if max_pages == 0 {
            DEFAULT_MAX_PAGES
        } else {
            max_pages.min(DEFAULT_MAX_PAGES)
        };
        Self {
            max_pages,
            status: STATUS_IDLE,
            load_err: LOAD_ERR_OK,
            img_len: 0,
            staging: Vec::new(),
            vm: None,
            exit_kind: 0,
            fault_detail: 0,
            cycles: 0,
            mem_high_water: 0,
            caps_at_exit: 0,
            out: Vec::new(),
            out_read_idx: 0,
            code_digest: [0; 32],
            out_digest: [0; 32],
            zero_verified: 0,
        }
    }

    pub fn status(&self) -> u32 {
        self.status
    }

    pub fn zero_verified(&self) -> u32 {
        self.zero_verified
    }

    pub fn magic_probe(&self) -> bool {
        self.peek_u32(OFF_MAGIC) == MAGIC
    }

    /// Read-only peek (no OUT_DATA cursor side effects).
    pub fn peek_u32(&self, offset: u32) -> u32 {
        match offset {
            OFF_MAGIC => MAGIC,
            OFF_VERSION => VERSION,
            OFF_HW_CAPS => (self.max_pages as u32) | ((MAX_CODE_BYTES as u32) << 16),
            OFF_STATUS => self.status,
            OFF_LOAD_ERR => self.load_err,
            OFF_IMG_LEN => self.img_len,
            OFF_EXIT_KIND => self.exit_kind,
            OFF_FAULT_DETAIL => self.fault_detail,
            OFF_CYCLES_LO => (self.cycles & 0xffff_ffff) as u32,
            OFF_CYCLES_HI => (self.cycles >> 32) as u32,
            OFF_OUT_LEN => self.out.len() as u32,
            OFF_OUT_READ_IDX => self.out_read_idx,
            OFF_ZERO_VERIFIED => self.zero_verified,
            OFF_MEM_HIGH_WATER => self.mem_high_water as u32,
            _ => 0,
        }
    }

    pub fn read_u8(&mut self, offset: u32) -> u8 {
        match offset {
            OFF_CAPS_AT_EXIT => self.caps_at_exit,
            OFF_OUT_DATA => self.read_out_byte(),
            o if (OFF_CODE_DIGEST..OFF_CODE_DIGEST + 32).contains(&o) => {
                self.code_digest[(o - OFF_CODE_DIGEST) as usize]
            }
            o if (OFF_OUT_DIGEST..OFF_OUT_DIGEST + 32).contains(&o) => {
                self.out_digest[(o - OFF_OUT_DIGEST) as usize]
            }
            o if (OFF_MEM_HIGH_WATER..OFF_MEM_HIGH_WATER + 2).contains(&o) => {
                self.mem_high_water.to_le_bytes()[(o - OFF_MEM_HIGH_WATER) as usize]
            }
            // Unaligned byte views of 32-bit RO regs (little-endian).
            o if o < MMIO_SIZE => {
                let base = o & !3;
                let byte = (o & 3) as usize;
                self.read_u32(base).to_le_bytes()[byte]
            }
            _ => 0,
        }
    }

    pub fn read_u16(&mut self, offset: u32) -> u16 {
        if offset == OFF_MEM_HIGH_WATER {
            return self.mem_high_water;
        }
        let lo = self.read_u8(offset) as u16;
        let hi = self.read_u8(offset.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    pub fn read_u32(&mut self, offset: u32) -> u32 {
        match offset {
            OFF_MAGIC => MAGIC,
            OFF_VERSION => VERSION,
            OFF_HW_CAPS => {
                (self.max_pages as u32) | ((MAX_CODE_BYTES as u32) << 16)
            }
            OFF_STATUS => self.status,
            OFF_LOAD_ERR => self.load_err,
            OFF_IMG_LEN => self.img_len,
            OFF_EXIT_KIND => self.exit_kind,
            OFF_FAULT_DETAIL => self.fault_detail,
            OFF_CYCLES_LO => (self.cycles & 0xffff_ffff) as u32,
            OFF_CYCLES_HI => (self.cycles >> 32) as u32,
            OFF_OUT_LEN => self.out.len() as u32,
            OFF_OUT_READ_IDX => self.out_read_idx,
            OFF_ZERO_VERIFIED => self.zero_verified,
            OFF_OUT_DATA => {
                // 32-bit read: up to 4 auto-incrementing OUT_DATA bytes.
                let mut buf = [0u8; 4];
                for b in &mut buf {
                    *b = self.read_out_byte();
                }
                u32::from_le_bytes(buf)
            }
            o if o == OFF_CODE_DIGEST
                || o == OFF_CODE_DIGEST + 4
                || o == OFF_CODE_DIGEST + 8
                || o == OFF_CODE_DIGEST + 12
                || o == OFF_CODE_DIGEST + 16
                || o == OFF_CODE_DIGEST + 20
                || o == OFF_CODE_DIGEST + 24
                || o == OFF_CODE_DIGEST + 28 =>
            {
                let i = (o - OFF_CODE_DIGEST) as usize;
                u32::from_le_bytes(self.code_digest[i..i + 4].try_into().unwrap())
            }
            o if o == OFF_OUT_DIGEST
                || o == OFF_OUT_DIGEST + 4
                || o == OFF_OUT_DIGEST + 8
                || o == OFF_OUT_DIGEST + 12
                || o == OFF_OUT_DIGEST + 16
                || o == OFF_OUT_DIGEST + 20
                || o == OFF_OUT_DIGEST + 24
                || o == OFF_OUT_DIGEST + 28 =>
            {
                let i = (o - OFF_OUT_DIGEST) as usize;
                u32::from_le_bytes(self.out_digest[i..i + 4].try_into().unwrap())
            }
            OFF_MEM_HIGH_WATER => self.mem_high_water as u32,
            _ => 0,
        }
    }

    pub fn write_u32(&mut self, offset: u32, value: u32) {
        match offset {
            OFF_CMD => self.strobe_cmd(value),
            OFF_IMG_LEN => {
                if matches!(self.status, STATUS_IDLE | STATUS_LOADING | STATUS_DISPOSED) {
                    self.img_len = value;
                }
            }
            OFF_IMG_FIFO => {
                if self.status == STATUS_LOADING {
                    self.staging.push((value & 0xff) as u8);
                }
            }
            OFF_OUT_READ_IDX => {
                if matches!(self.status, STATUS_DONE | STATUS_FAULTED) {
                    self.out_read_idx = value;
                }
            }
            _ => {}
        }
    }

    pub fn write_u8(&mut self, offset: u32, value: u8) {
        match offset {
            OFF_IMG_FIFO if self.status == STATUS_LOADING => {
                self.staging.push(value);
            }
            _ => {
                // Widen to 32-bit write for CMD / IMG_LEN / OUT_READ_IDX when aligned.
                if offset & 3 == 0 {
                    self.write_u32(offset, value as u32);
                }
            }
        }
    }

    /// Stream raw BIL1 bytes into `IMG_FIFO` (host helper; guest would loop MMIO).
    pub fn stream_image(&mut self, bil1: &[u8]) {
        for &b in bil1 {
            self.write_u32(OFF_IMG_FIFO, b as u32);
        }
    }

    fn read_out_byte(&mut self) -> u8 {
        let idx = self.out_read_idx as usize;
        if idx < self.out.len() {
            let b = self.out[idx];
            self.out_read_idx = self.out_read_idx.wrapping_add(1);
            b
        } else {
            0
        }
    }

    fn strobe_cmd(&mut self, cmd: u32) {
        match cmd {
            CMD_BEGIN_LOAD => self.cmd_begin_load(),
            CMD_COMMIT_LOAD => self.cmd_commit_load(),
            CMD_STEP => self.cmd_step(),
            CMD_RUN => self.cmd_run(),
            CMD_RESET_ZEROIZE => self.cmd_reset_zeroize(),
            _ => {}
        }
    }

    fn cmd_begin_load(&mut self) {
        match self.status {
            STATUS_IDLE | STATUS_DISPOSED => {
                self.staging.clear();
                self.staging.reserve(self.img_len as usize);
                self.load_err = LOAD_ERR_OK;
                self.zero_verified = 0;
                self.clear_result_window();
                self.vm = None;
                self.status = STATUS_LOADING;
            }
            _ => {
                self.load_err = LOAD_ERR_BUSY;
            }
        }
    }

    fn cmd_commit_load(&mut self) {
        if self.status != STATUS_LOADING {
            self.load_err = LOAD_ERR_BUSY;
            return;
        }
        if self.staging.len() as u32 != self.img_len {
            self.load_err = LOAD_ERR_LEN;
            // Remain non-runnable (stay Loading / not Loaded).
            return;
        }
        let img = match BytecodeImage::decode(&self.staging) {
            Ok(img) => img,
            Err(_) => {
                self.load_err = LOAD_ERR_IMAGE;
                return;
            }
        };
        if img.budget.max_mem_pages == 0 || img.budget.max_mem_pages > self.max_pages {
            self.load_err = LOAD_ERR_PAGES;
            return;
        }
        let vm = match Vm::new(&img.code, img.entry, img.budget, img.initial_caps) {
            Ok(vm) => vm,
            Err(_) => {
                self.load_err = LOAD_ERR_VM_INIT;
                return;
            }
        };
        self.code_digest = vm.code_digest;
        self.vm = Some(vm);
        self.load_err = LOAD_ERR_OK;
        self.status = STATUS_LOADED;
    }

    fn cmd_step(&mut self) {
        if !matches!(self.status, STATUS_LOADED | STATUS_RUNNING) {
            return;
        }
        self.status = STATUS_RUNNING;
        let Some(vm) = self.vm.as_mut() else {
            return;
        };
        match vm.step() {
            Ok(None) => {}
            Ok(Some(reason)) => self.latch_exit(reason),
            Err(kind) => self.latch_exit(ExitReason::Fault(kind)),
        }
    }

    fn cmd_run(&mut self) {
        if self.status != STATUS_LOADED {
            return;
        }
        self.status = STATUS_RUNNING;
        let Some(mut vm) = self.vm.take() else {
            return;
        };
        let result = vm.run();
        self.vm = Some(vm);
        self.latch_from_result(&result);
    }

    fn latch_from_result(&mut self, result: &bilkrnl_core::isa::EnclaveResult) {
        self.exit_kind = match result.exit {
            ExitReason::Halt => EXIT_HALT,
            ExitReason::BudgetExceeded => EXIT_BUDGET_EXCEEDED,
            ExitReason::Fault(_) => EXIT_FAULT,
        };
        self.fault_detail = match result.exit {
            ExitReason::Fault(kind) => pack_fault(kind),
            _ => 0,
        };
        self.cycles = result.cycles;
        self.mem_high_water = result.mem_high_water;
        self.caps_at_exit = result.caps_at_exit.as_u8();
        self.out = result.out.clone();
        self.out_read_idx = 0;
        self.code_digest = result.code_digest;
        self.out_digest = result.out_digest;
        self.status = match result.exit {
            ExitReason::Fault(_) => STATUS_FAULTED,
            _ => STATUS_DONE,
        };
    }

    fn latch_exit(&mut self, reason: ExitReason) {
        let Some(vm) = self.vm.as_ref() else {
            return;
        };
        let out_digest = bilkrnl_core::digest::sha256(&vm.out);
        let result = bilkrnl_core::isa::EnclaveResult {
            exit: reason,
            out: vm.out.clone(),
            cycles: vm.account.cycles,
            mem_high_water: vm.account.mem_high_water_pages,
            code_digest: vm.code_digest,
            out_digest,
            caps_at_exit: vm.caps,
        };
        self.latch_from_result(&result);
    }

    fn cmd_reset_zeroize(&mut self) {
        match self.status {
            STATUS_IDLE => {
                // DisposeError::NotFound analogue — stay Idle, ZERO_VERIFIED = 0.
                self.zero_verified = 0;
            }
            STATUS_LOADING | STATUS_RUNNING => {
                // Reject mid-flight (same spirit as busy dispose).
            }
            STATUS_DISPOSED => {
                // Idempotent.
                self.zero_verified = 1;
            }
            STATUS_LOADED | STATUS_DONE | STATUS_FAULTED => {
                if let Some(mut vm) = self.vm.take() {
                    vm.zeroize();
                    // Verify wipe.
                    debug_assert!(vm.mem.iter().all(|&b| b == 0));
                    debug_assert!(vm.out.is_empty());
                }
                self.staging.fill(0);
                self.staging.clear();
                self.clear_result_window();
                self.img_len = 0;
                self.load_err = LOAD_ERR_OK;
                self.zero_verified = 1;
                self.status = STATUS_DISPOSED;
            }
            _ => {}
        }
    }

    fn clear_result_window(&mut self) {
        self.exit_kind = 0;
        self.fault_detail = 0;
        self.cycles = 0;
        self.mem_high_water = 0;
        self.caps_at_exit = 0;
        self.out.fill(0);
        self.out.clear();
        self.out_read_idx = 0;
        self.code_digest = [0; 32];
        self.out_digest = [0; 32];
    }
}

pub fn pack_fault(kind: FaultKind) -> u32 {
    let (disc, payload) = match kind {
        FaultKind::IllegalOpcode(op) => (FAULT_ILLEGAL_OPCODE, op),
        FaultKind::PcOob => (FAULT_PC_OOB, 0),
        FaultKind::MemOob => (FAULT_MEM_OOB, 0),
        FaultKind::HostEscapeAttempt(op) => (FAULT_HOST_ESCAPE, op),
        FaultKind::MissingCap => (FAULT_MISSING_CAP, 0),
        FaultKind::OutOverflow => (FAULT_OUT_OVERFLOW, 0),
    };
    ((disc as u32) << 8) | (payload as u32)
}

pub fn unpack_fault(detail: u32) -> FaultKind {
    let payload = (detail & 0xff) as u8;
    let disc = ((detail >> 8) & 0xff) as u8;
    match disc {
        FAULT_ILLEGAL_OPCODE => FaultKind::IllegalOpcode(payload),
        FAULT_PC_OOB => FaultKind::PcOob,
        FAULT_MEM_OOB => FaultKind::MemOob,
        FAULT_HOST_ESCAPE => FaultKind::HostEscapeAttempt(payload),
        FAULT_MISSING_CAP => FaultKind::MissingCap,
        FAULT_OUT_OVERFLOW => FaultKind::OutOverflow,
        _ => FaultKind::IllegalOpcode(payload),
    }
}

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn magic_and_version_probe() {
        let mut d = SoftCpu::new(256);
        assert_eq!(d.read_u32(OFF_MAGIC), MAGIC);
        assert_eq!(d.read_u32(OFF_VERSION), VERSION);
        assert!(d.magic_probe());
    }
}
