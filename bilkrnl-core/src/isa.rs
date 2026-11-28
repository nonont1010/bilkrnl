//! 6502-style soft ISA: linear address space, direct step, no speculative paths.

use alloc::vec::Vec;

use crate::budget::{BudgetAccount, BudgetError, ResourceBudget};
use crate::cap::Cap;
use crate::digest::sha256;

pub const PAGE_SIZE: usize = 4096;
pub const MAX_LINEAR_BYTES: usize = 256 * PAGE_SIZE; // 1 MiB hard ceiling

/// Exit reasons reported to the blast chamber.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitReason {
    Halt,
    BudgetExceeded,
    Fault(FaultKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultKind {
    IllegalOpcode(u8),
    PcOob,
    MemOob,
    HostEscapeAttempt(u8),
    MissingCap,
    OutOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnclaveResult {
    pub exit: ExitReason,
    pub out: Vec<u8>,
    pub cycles: u64,
    pub mem_high_water: u16,
    pub code_digest: [u8; 32],
    pub out_digest: [u8; 32],
    pub caps_at_exit: Cap,
}

/// Soft machine: accumulator + PC + linear RAM + bounded out buffer.
#[derive(Debug)]
pub struct Vm {
    pub mem: Vec<u8>,
    pub pc: u32,
    pub acc: u8,
    pub out: Vec<u8>,
    pub account: BudgetAccount,
    pub budget: ResourceBudget,
    pub caps: Cap,
    pub code_digest: [u8; 32],
    pub code_len: u32,
}

/// Opcodes. Values in 0xE0..=0xEF are reserved "host/EFI shaped" traps (TC-08).
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Op {
    Halt = 0x00,
    Nop = 0x01,
    LoadImm = 0x02,  // ACC <- imm8
    Load = 0x03,     // ACC <- mem[u16]
    Store = 0x04,    // mem[u16] <- ACC
    AddImm = 0x05,   // ACC <- ACC + imm8
    SubImm = 0x06,   // ACC <- ACC - imm8
    Jmp = 0x07,      // PC <- u16
    Jz = 0x08,       // if ACC==0 PC <- u16
    Out = 0x09,      // append ACC to out (needs IoOutBounded)
    Weaken = 0x0A,   // weaken caps to imm8 Cap
}

impl Vm {
    pub fn new(
        code: &[u8],
        entry: u32,
        budget: ResourceBudget,
        caps: Cap,
    ) -> Result<Self, FaultKind> {
        let mem_bytes = (budget.max_mem_pages as usize).saturating_mul(PAGE_SIZE);
        if mem_bytes == 0 || mem_bytes > MAX_LINEAR_BYTES {
            return Err(FaultKind::MemOob);
        }
        if entry as usize >= code.len() && !code.is_empty() {
            return Err(FaultKind::PcOob);
        }
        if code.len() > mem_bytes {
            return Err(FaultKind::MemOob);
        }
        let mut mem = alloc::vec![0u8; mem_bytes];
        mem[..code.len()].copy_from_slice(code);
        let code_digest = sha256(code);
        Ok(Self {
            mem,
            pc: entry,
            acc: 0,
            out: Vec::new(),
            account: BudgetAccount::default(),
            budget,
            caps,
            code_digest,
            code_len: code.len() as u32,
        })
    }

    fn read_u8(&mut self) -> Result<u8, FaultKind> {
        let pc = self.pc as usize;
        if pc >= self.mem.len() {
            return Err(FaultKind::PcOob);
        }
        let b = self.mem[pc];
        self.pc = self.pc.wrapping_add(1);
        Ok(b)
    }

    fn read_u16(&mut self) -> Result<u16, FaultKind> {
        let lo = self.read_u8()? as u16;
        let hi = self.read_u8()? as u16;
        Ok(lo | (hi << 8))
    }

    fn mem_load(&mut self, addr: u16) -> Result<u8, FaultKind> {
        let a = addr as u32;
        self.account
            .note_addr(a, &self.budget)
            .map_err(|_| FaultKind::MemOob)?;
        let idx = a as usize;
        if idx >= self.mem.len() {
            return Err(FaultKind::MemOob);
        }
        Ok(self.mem[idx])
    }

    fn mem_store(&mut self, addr: u16, val: u8) -> Result<(), FaultKind> {
        let a = addr as u32;
        self.account
            .note_addr(a, &self.budget)
            .map_err(|_| FaultKind::MemOob)?;
        let idx = a as usize;
        if idx >= self.mem.len() {
            return Err(FaultKind::MemOob);
        }
        self.mem[idx] = val;
        Ok(())
    }

    /// Execute exactly one instruction. Commits or faults — no alternate-path prefetch.
    pub fn step(&mut self) -> Result<Option<ExitReason>, FaultKind> {
        match self.account.charge_cycle(&self.budget) {
            Err(BudgetError::CyclesExceeded { .. }) => return Ok(Some(ExitReason::BudgetExceeded)),
            Err(_) => unreachable!(),
            Ok(()) => {}
        }

        if self.caps < Cap::Compute {
            return Err(FaultKind::MissingCap);
        }

        let op = self.read_u8()?;

        // Host/EFI-shaped opcodes: hard fault, never side-effect host.
        if (0xE0..=0xEF).contains(&op) {
            return Err(FaultKind::HostEscapeAttempt(op));
        }

        match op {
            x if x == Op::Halt as u8 => Ok(Some(ExitReason::Halt)),
            x if x == Op::Nop as u8 => Ok(None),
            x if x == Op::LoadImm as u8 => {
                self.acc = self.read_u8()?;
                Ok(None)
            }
            x if x == Op::Load as u8 => {
                let addr = self.read_u16()?;
                self.acc = self.mem_load(addr)?;
                Ok(None)
            }
            x if x == Op::Store as u8 => {
                let addr = self.read_u16()?;
                self.mem_store(addr, self.acc)?;
                Ok(None)
            }
            x if x == Op::AddImm as u8 => {
                let imm = self.read_u8()?;
                self.acc = self.acc.wrapping_add(imm);
                Ok(None)
            }
            x if x == Op::SubImm as u8 => {
                let imm = self.read_u8()?;
                self.acc = self.acc.wrapping_sub(imm);
                Ok(None)
            }
            x if x == Op::Jmp as u8 => {
                let addr = self.read_u16()?;
                if addr as usize >= self.mem.len() {
                    return Err(FaultKind::PcOob);
                }
                self.pc = addr as u32;
                Ok(None)
            }
            x if x == Op::Jz as u8 => {
                let addr = self.read_u16()?;
                if self.acc == 0 {
                    if addr as usize >= self.mem.len() {
                        return Err(FaultKind::PcOob);
                    }
                    self.pc = addr as u32;
                }
                Ok(None)
            }
            x if x == Op::Out as u8 => {
                if self.caps < Cap::IoOutBounded {
                    return Err(FaultKind::MissingCap);
                }
                match self.account.charge_out(1, &self.budget) {
                    Err(BudgetError::OutBytesExceeded { .. }) => {
                        return Err(FaultKind::OutOverflow);
                    }
                    Err(_) => unreachable!(),
                    Ok(()) => {}
                }
                self.out.push(self.acc);
                Ok(None)
            }
            x if x == Op::Weaken as u8 => {
                let raw = self.read_u8()?;
                let to = Cap::from_u8(raw).ok_or(FaultKind::IllegalOpcode(raw))?;
                self.caps = crate::cap::CapPoset::weaken(self.caps, to)
                    .map_err(|_| FaultKind::MissingCap)?;
                Ok(None)
            }
            other => Err(FaultKind::IllegalOpcode(other)),
        }
    }

    pub fn run(&mut self) -> EnclaveResult {
        let exit = loop {
            match self.step() {
                Ok(Some(reason)) => break reason,
                Ok(None) => continue,
                Err(kind) => break ExitReason::Fault(kind),
            }
        };
        let out_digest = sha256(&self.out);
        EnclaveResult {
            exit,
            out: self.out.clone(),
            cycles: self.account.cycles,
            mem_high_water: self.account.mem_high_water_pages,
            code_digest: self.code_digest,
            out_digest,
            caps_at_exit: self.caps,
        }
    }

    /// Zeroize linear memory and out buffer (dispose helper).
    pub fn zeroize(&mut self) {
        for b in self.mem.iter_mut() {
            *b = 0;
        }
        for b in self.out.iter_mut() {
            *b = 0;
        }
        self.out.clear();
        self.acc = 0;
        self.pc = 0;
        self.caps = Cap::None;
        self.account = BudgetAccount::default();
    }
}

/// Assemble a tiny "write OK\\n and Halt" fixture used by e2e tests.
pub fn fixture_ok_program() -> Vec<u8> {
    // LOAD_IMM 'O'; OUT; LOAD_IMM 'K'; OUT; LOAD_IMM '\n'; OUT; HALT
    alloc::vec![
        Op::LoadImm as u8,
        b'O',
        Op::Out as u8,
        Op::LoadImm as u8,
        b'K',
        Op::Out as u8,
        Op::LoadImm as u8,
        b'\n',
        Op::Out as u8,
        Op::Halt as u8,
    ]
}

/// Sealed policy-gate image (PASS): candidate `0x2A` at `@0`, entry=1.
///
/// Bytes match `tools/bil1asm/examples/policy_pass.bil1.ir` (B3.1). SoftISA needs
/// `Compute` to execute `Halt`, so the program weakens `IoOutBounded → Compute`.
pub fn fixture_policy_pass_program() -> (u32, Vec<u8>) {
    (
        1,
        alloc::vec![
            0x2a, // candidate
            0x03, 0x00, 0x00, 0x06, 0x2a, 0x08, 0x2b, 0x00, // ld @0; subi 0x2A; jz pass
            0x03, 0x00, 0x00, 0x06, 0x55, 0x08, 0x2b, 0x00, // ld @0; subi 0x55; jz pass
            0x03, 0x00, 0x00, 0x06, 0x7f, 0x08, 0x2b, 0x00, // ld @0; subi 0x7F; jz pass
            0x02, 0x46, 0x09, 0x02, 0x41, 0x09, 0x02, 0x49, 0x09, 0x02, 0x4c, 0x09, 0x02, 0x0a,
            0x09, // FAIL\n
            0x0a, 0x01, 0x00, // weaken Compute; halt
            0x02, 0x50, 0x09, 0x02, 0x41, 0x09, 0x02, 0x53, 0x09, 0x02, 0x53, 0x09, 0x02, 0x0a,
            0x09, // PASS\n
            0x0a, 0x01, 0x00, // weaken Compute; halt
        ],
    )
}

/// Sealed policy-gate image (FAIL): candidate `0x00` at `@0`, same code as PASS twin.
pub fn fixture_policy_fail_program() -> (u32, Vec<u8>) {
    let (entry, mut code) = fixture_policy_pass_program();
    code[0] = 0x00;
    (entry, code)
}

/// Sealed attested-reverse workload (PASS): four sealed bytes `@0..@3` = `BIL!`,
/// entry=4; program Outs them in reverse (`!LIB`) then weakens to Compute and Halts.
///
/// Bytes match `tools/bil1asm/examples/rev4_pass.bil1.ir`. Digests bind the sealed
/// payload via `code_digest`; a consumer ratifies ExactBytes(`!LIB`).
pub fn fixture_rev4_pass_program() -> (u32, Vec<u8>) {
    (
        4,
        alloc::vec![
            b'B', b'I', b'L', b'!', // sealed payload @0..@3
            // ld @3; out; ld @2; out; ld @1; out; ld @0; out
            0x03, 0x03, 0x00, 0x09, // ld @3; out
            0x03, 0x02, 0x00, 0x09, // ld @2; out
            0x03, 0x01, 0x00, 0x09, // ld @1; out
            0x03, 0x00, 0x00, 0x09, // ld @0; out
            0x0a, 0x01, // weaken Compute
            0x00, // halt
        ],
    )
}

/// Negative twin: sealed `XXXX` → Out `XXXX` (not `!LIB`).
pub fn fixture_rev4_fail_program() -> (u32, Vec<u8>) {
    let (entry, mut code) = fixture_rev4_pass_program();
    code[0] = b'X';
    code[1] = b'X';
    code[2] = b'X';
    code[3] = b'X';
    (entry, code)
}
