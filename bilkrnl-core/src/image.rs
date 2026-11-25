//! BIL1 bytecode image parse/serialize contract.

use alloc::vec::Vec;

use crate::budget::ResourceBudget;
use crate::cap::{Cap, CapError};

pub const IMAGE_MAGIC: [u8; 4] = *b"BIL1";
pub const HEADER_LEN: usize = 32;
pub const MAX_CODE_LEN: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeImage {
    pub entry: u32,
    pub code: Vec<u8>,
    pub initial_caps: Cap,
    pub budget: ResourceBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageError {
    TooShort,
    BadMagic([u8; 4]),
    Truncated { expected: usize, got: usize },
    EntryOob { entry: u32, code_len: u32 },
    Cap(CapError),
    CodeTooLarge(usize),
    ZeroMemPages,
}

impl BytecodeImage {
    pub fn new(
        entry: u32,
        code: Vec<u8>,
        initial_caps: Cap,
        budget: ResourceBudget,
    ) -> Result<Self, ImageError> {
        if budget.max_mem_pages == 0 {
            return Err(ImageError::ZeroMemPages);
        }
        if code.len() > MAX_CODE_LEN {
            return Err(ImageError::CodeTooLarge(code.len()));
        }
        if !code.is_empty() && entry as usize >= code.len() {
            return Err(ImageError::EntryOob {
                entry,
                code_len: code.len() as u32,
            });
        }
        Ok(Self {
            entry,
            code,
            initial_caps,
            budget,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.code.len());
        out.extend_from_slice(&IMAGE_MAGIC);
        out.extend_from_slice(&self.entry.to_le_bytes());
        out.extend_from_slice(&(self.code.len() as u32).to_le_bytes());
        out.push(self.initial_caps.as_u8());
        out.extend_from_slice(&[0u8; 3]); // reserved
        out.extend_from_slice(&self.budget.max_cycles.to_le_bytes());
        out.extend_from_slice(&self.budget.max_out_bytes.to_le_bytes());
        out.extend_from_slice(&self.budget.max_mem_pages.to_le_bytes());
        out.extend_from_slice(&[0u8; 2]); // pad to 32
        out.extend_from_slice(&self.code);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ImageError> {
        if bytes.len() < HEADER_LEN {
            return Err(ImageError::TooShort);
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if magic != IMAGE_MAGIC {
            return Err(ImageError::BadMagic(magic));
        }
        let entry = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let code_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let caps_raw = bytes[12];
        let initial_caps =
            Cap::from_u8(caps_raw).ok_or(ImageError::Cap(CapError::UnknownCap(caps_raw)))?;
        let max_cycles = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let max_out_bytes = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let max_mem_pages = u16::from_le_bytes(bytes[28..30].try_into().unwrap());
        let expected = HEADER_LEN + code_len;
        if bytes.len() < expected {
            return Err(ImageError::Truncated {
                expected,
                got: bytes.len(),
            });
        }
        if code_len > MAX_CODE_LEN {
            return Err(ImageError::CodeTooLarge(code_len));
        }
        let code = bytes[HEADER_LEN..HEADER_LEN + code_len].to_vec();
        Self::new(
            entry,
            code,
            initial_caps,
            ResourceBudget::new(max_cycles, max_out_bytes, max_mem_pages),
        )
    }
}
