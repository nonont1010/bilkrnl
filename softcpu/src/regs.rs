//! MMIO register map — mirrors `softcpu/include/bil_softcpu.h` / `DEVICE.md`.
//! The C header stays uncompiled (`#error` under `BIL_SOFTCPU_BUILD`).

pub const MMIO_SIZE: u32 = 0x1_0000;

pub const OFF_MAGIC: u32 = 0x0000;
pub const OFF_VERSION: u32 = 0x0004;
pub const OFF_HW_CAPS: u32 = 0x0008;
pub const OFF_STATUS: u32 = 0x000C;
pub const OFF_CMD: u32 = 0x0010;
pub const OFF_LOAD_ERR: u32 = 0x0014;
pub const OFF_IMG_LEN: u32 = 0x0018;
pub const OFF_IMG_FIFO: u32 = 0x001C;
pub const OFF_EXIT_KIND: u32 = 0x0020;
pub const OFF_FAULT_DETAIL: u32 = 0x0024;
pub const OFF_CYCLES_LO: u32 = 0x0028;
pub const OFF_CYCLES_HI: u32 = 0x002C;
pub const OFF_MEM_HIGH_WATER: u32 = 0x0030;
pub const OFF_CAPS_AT_EXIT: u32 = 0x0032;
pub const OFF_OUT_LEN: u32 = 0x0034;
pub const OFF_OUT_READ_IDX: u32 = 0x0038;
pub const OFF_OUT_DATA: u32 = 0x003C;
pub const OFF_CODE_DIGEST: u32 = 0x0040;
pub const OFF_OUT_DIGEST: u32 = 0x0060;
pub const OFF_ZERO_VERIFIED: u32 = 0x0080;

pub const MAGIC: u32 = 0x314C_4942; // "BIL1" LE
pub const VERSION: u32 = 0x0001_0000;
pub const DEFAULT_MAX_PAGES: u16 = 256;
pub const MAX_CODE_BYTES: u16 = 65_535;

pub const STATUS_IDLE: u32 = 0;
pub const STATUS_LOADING: u32 = 1;
pub const STATUS_LOADED: u32 = 2;
pub const STATUS_RUNNING: u32 = 3;
pub const STATUS_DONE: u32 = 4;
pub const STATUS_FAULTED: u32 = 5;
pub const STATUS_DISPOSED: u32 = 6;

pub const CMD_BEGIN_LOAD: u32 = 1;
pub const CMD_COMMIT_LOAD: u32 = 2;
pub const CMD_STEP: u32 = 3;
pub const CMD_RUN: u32 = 4;
pub const CMD_RESET_ZEROIZE: u32 = 5;

pub const EXIT_HALT: u32 = 0;
pub const EXIT_BUDGET_EXCEEDED: u32 = 1;
pub const EXIT_FAULT: u32 = 2;

pub const FAULT_ILLEGAL_OPCODE: u8 = 0;
pub const FAULT_PC_OOB: u8 = 1;
pub const FAULT_MEM_OOB: u8 = 2;
pub const FAULT_HOST_ESCAPE: u8 = 3;
pub const FAULT_MISSING_CAP: u8 = 4;
pub const FAULT_OUT_OVERFLOW: u8 = 5;

/// `LOAD_ERR` diagnostic codes (host-sim local; not part of BIL1).
pub const LOAD_ERR_OK: u32 = 0;
pub const LOAD_ERR_IMAGE: u32 = 1;
pub const LOAD_ERR_VM_INIT: u32 = 2;
pub const LOAD_ERR_BUSY: u32 = 3;
pub const LOAD_ERR_LEN: u32 = 4;
pub const LOAD_ERR_PAGES: u32 = 5;
