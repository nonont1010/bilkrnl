//! Guest MMIO client for QEMU `-device bil-softcpu`.
//!
//! Probe: EFI configuration table DTB → `compatible = "bil,softcpu"` → `reg`.
//! Fallback: interpreter (`blast`) when the device is absent.

use bilkrnl_core::cap::Cap;
use bilkrnl_core::image::BytecodeImage;
use bilkrnl_core::isa::{EnclaveResult, ExitReason, FaultKind};
use core::ptr::{read_volatile, write_volatile};
use uefi::table::cfg::ConfigTableEntry;
use uefi::{guid, Guid};

/// EFI Device Tree Table (Linux/QEMU AAVMF).
const EFI_DTB_TABLE_GUID: Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");

const OFF_MAGIC: u32 = 0x0000;
const OFF_STATUS: u32 = 0x000C;
const OFF_CMD: u32 = 0x0010;
const OFF_LOAD_ERR: u32 = 0x0014;
const OFF_IMG_LEN: u32 = 0x0018;
const OFF_IMG_FIFO: u32 = 0x001C;
const OFF_EXIT_KIND: u32 = 0x0020;
const OFF_FAULT_DETAIL: u32 = 0x0024;
const OFF_CYCLES_LO: u32 = 0x0028;
const OFF_CYCLES_HI: u32 = 0x002C;
const OFF_MEM_HIGH_WATER: u32 = 0x0030;
const OFF_CAPS_AT_EXIT: u32 = 0x0032;
const OFF_OUT_LEN: u32 = 0x0034;
const OFF_OUT_READ_IDX: u32 = 0x0038;
const OFF_OUT_DATA: u32 = 0x003C;
const OFF_CODE_DIGEST: u32 = 0x0040;
const OFF_OUT_DIGEST: u32 = 0x0060;
const OFF_ZERO_VERIFIED: u32 = 0x0080;

const MAGIC: u32 = 0x314C_4942;
const CMD_BEGIN_LOAD: u32 = 1;
const CMD_COMMIT_LOAD: u32 = 2;
const CMD_RUN: u32 = 4;
const CMD_RESET_ZEROIZE: u32 = 5;
const STATUS_DONE: u32 = 4;
const STATUS_FAULTED: u32 = 5;
const STATUS_DISPOSED: u32 = 6;

struct Mmio(u64);

impl Mmio {
    unsafe fn r32(&self, off: u32) -> u32 {
        read_volatile((self.0 + off as u64) as *const u32)
    }
    unsafe fn w32(&self, off: u32, v: u32) {
        write_volatile((self.0 + off as u64) as *mut u32, v);
    }
    unsafe fn r8(&self, off: u32) -> u8 {
        read_volatile((self.0 + off as u64) as *const u8)
    }
}

/// Well-known platform-bus window on QEMU `virt` (see softcpu/qemu-device.md).
const PLATFORM_BUS_BASE: u64 = 0x0c00_0000;

/// Locate `bil-softcpu` MMIO base: MAGIC probe at platform-bus +0, else EFI DTB.
pub fn probe_mmio_base() -> Option<u64> {
    // Fast path: forked QEMU places the first dynamic sysbus device at
    // platform-bus@c000000 + child reg 0 (bil-softcpu@0 → 0x0c000000).
    let guess = PLATFORM_BUS_BASE;
    let mmio = Mmio(guess);
    if unsafe { mmio.r32(OFF_MAGIC) } == MAGIC {
        return Some(guess);
    }
    let dtb = find_dtb()?;
    let base = find_softcpu_reg(dtb)?;
    let mmio = Mmio(base);
    if unsafe { mmio.r32(OFF_MAGIC) } == MAGIC {
        Some(base)
    } else {
        None
    }
}

fn find_dtb() -> Option<&'static [u8]> {
    let ptr = uefi::system::with_config_table(|entries: &[ConfigTableEntry]| {
        entries
            .iter()
            .find(|e| e.guid == EFI_DTB_TABLE_GUID)
            .map(|e| e.address as *const u8)
    })?;
    // FDT header totalsize at +4 (BE).
    let hdr = unsafe { core::slice::from_raw_parts(ptr, 8) };
    if hdr[0..4] != [0xd0, 0x0d, 0xfe, 0xed] {
        return None;
    }
    let totalsize = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
    if totalsize < 32 || totalsize > 2 * 1024 * 1024 {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(ptr, totalsize) })
}

fn find_softcpu_reg(dtb: &[u8]) -> Option<u64> {
    // Minimal FDT v17 walker: find compatible=bil,softcpu, then reg; resolve via
    // ancestor platform-bus ranges (child #address-cells=1 → parent 0x0c000000).
    if dtb.len() < 28 {
        return None;
    }
    let off_struct = u32::from_be_bytes(dtb[8..12].try_into().ok()?) as usize;
    let off_strings = u32::from_be_bytes(dtb[12..16].try_into().ok()?) as usize;
    let mut pos = off_struct;
    let mut path_depth = 0i32;
    let mut in_target = false;
    let mut target_depth = -1i32;
    let mut child_reg_addr: Option<u32> = None;
    let mut platform_base: Option<u64> = None;

    while pos + 4 <= dtb.len() {
        let tag = u32::from_be_bytes(dtb[pos..pos + 4].try_into().ok()?);
        pos += 4;
        match tag {
            1 => {
                // BEGIN_NODE
                let end = dtb[pos..].iter().position(|&b| b == 0)? + pos;
                let name = core::str::from_utf8(&dtb[pos..end]).unwrap_or("");
                pos = (end + 4) & !3;
                path_depth += 1;
                if name.starts_with("platform-bus@") {
                    // parse hex after @
                    if let Some(hex) = name.split('@').nth(1) {
                        platform_base = u64::from_str_radix(hex, 16).ok();
                    }
                }
            }
            2 => {
                // END_NODE
                if in_target && path_depth == target_depth {
                    in_target = false;
                }
                path_depth -= 1;
            }
            3 => {
                // PROP
                if pos + 8 > dtb.len() {
                    break;
                }
                let plen = u32::from_be_bytes(dtb[pos..pos + 4].try_into().ok()?) as usize;
                let nameoff = u32::from_be_bytes(dtb[pos + 4..pos + 8].try_into().ok()?) as usize;
                pos += 8;
                let nstart = off_strings + nameoff;
                let nend = dtb[nstart..].iter().position(|&b| b == 0)? + nstart;
                let pname = core::str::from_utf8(&dtb[nstart..nend]).unwrap_or("");
                if pos + plen > dtb.len() {
                    break;
                }
                let pval = &dtb[pos..pos + plen];
                pos = (pos + plen + 3) & !3;

                if pname == "compatible" && pval.windows(11).any(|w| w == b"bil,softcpu") {
                    in_target = true;
                    target_depth = path_depth;
                }
                if in_target && path_depth == target_depth && pname == "reg" && pval.len() >= 4 {
                    child_reg_addr = Some(u32::from_be_bytes(pval[0..4].try_into().ok()?));
                }
            }
            4 => {}
            9 => break,
            _ => break,
        }
    }

    let child = child_reg_addr? as u64;
    let base = platform_base.unwrap_or(0x0c00_0000);
    Some(base + child)
}

/// Blast via MMIO device. Returns `None` if MAGIC probe fails.
pub fn blast_mmio(base: u64, img: &BytecodeImage) -> Option<EnclaveResult> {
    let mmio = Mmio(base);
    unsafe {
        if mmio.r32(OFF_MAGIC) != MAGIC {
            return None;
        }
        let bil1 = img.encode();
        mmio.w32(OFF_IMG_LEN, bil1.len() as u32);
        mmio.w32(OFF_CMD, CMD_BEGIN_LOAD);
        for b in &bil1 {
            mmio.w32(OFF_IMG_FIFO, *b as u32);
        }
        mmio.w32(OFF_CMD, CMD_COMMIT_LOAD);
        if mmio.r32(OFF_LOAD_ERR) != 0 {
            return None;
        }
        mmio.w32(OFF_CMD, CMD_RUN);
        let status = mmio.r32(OFF_STATUS);
        if status != STATUS_DONE && status != STATUS_FAULTED {
            return None;
        }

        let exit_kind = mmio.r32(OFF_EXIT_KIND);
        let exit = match exit_kind {
            0 => ExitReason::Halt,
            1 => ExitReason::BudgetExceeded,
            2 => {
                let detail = mmio.r32(OFF_FAULT_DETAIL);
                let disc = ((detail >> 8) & 0xff) as u8;
                let payload = (detail & 0xff) as u8;
                let fault = match disc {
                    0 => FaultKind::IllegalOpcode(payload),
                    1 => FaultKind::PcOob,
                    2 => FaultKind::MemOob,
                    3 => FaultKind::HostEscapeAttempt(payload),
                    4 => FaultKind::MissingCap,
                    5 => FaultKind::OutOverflow,
                    _ => FaultKind::IllegalOpcode(payload),
                };
                ExitReason::Fault(fault)
            }
            _ => return None,
        };

        let cycles = (mmio.r32(OFF_CYCLES_LO) as u64) | ((mmio.r32(OFF_CYCLES_HI) as u64) << 32);
        let mem_high_water = (mmio.r32(OFF_MEM_HIGH_WATER) & 0xffff) as u16;
        // CAPS is at 0x32 — read via byte.
        let caps_raw = mmio.r8(OFF_CAPS_AT_EXIT);
        let caps_at_exit = Cap::from_u8(caps_raw)?;

        let out_len = mmio.r32(OFF_OUT_LEN) as usize;
        mmio.w32(OFF_OUT_READ_IDX, 0);
        let mut out = alloc::vec::Vec::with_capacity(out_len);
        for _ in 0..out_len {
            out.push(mmio.r8(OFF_OUT_DATA));
        }

        let mut code_digest = [0u8; 32];
        for (i, b) in code_digest.iter_mut().enumerate() {
            *b = mmio.r8(OFF_CODE_DIGEST + i as u32);
        }
        let mut out_digest = [0u8; 32];
        for (i, b) in out_digest.iter_mut().enumerate() {
            *b = mmio.r8(OFF_OUT_DIGEST + i as u32);
        }
        mmio.w32(OFF_CMD, CMD_RESET_ZEROIZE);
        if mmio.r32(OFF_STATUS) != STATUS_DISPOSED || mmio.r32(OFF_ZERO_VERIFIED) != 1 {
            return None;
        }

        Some(EnclaveResult {
            exit,
            out,
            cycles,
            mem_high_water,
            code_digest,
            out_digest,
            caps_at_exit,
        })
    }
}

/// Run image on softcpu if present, else `None` (caller falls back to interpreter).
pub fn try_blast(img: &BytecodeImage) -> Option<EnclaveResult> {
    let base = probe_mmio_base()?;
    blast_mmio(base, img)
}
