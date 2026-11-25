//! Capability poset with forward-only ratchet (weaken only).

use core::fmt;

/// Capability lattice ordered weakest → strongest.
///
/// Ratchet rule: a live capability may only move to a *weaker* (or equal) value.
/// EFI / host calls are intentionally absent — not grantable to enclaves in V1.
#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Cap {
    None = 0,
    Compute = 1,
    IoOutBounded = 2,
}

impl Cap {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Cap::None),
            1 => Some(Cap::Compute),
            2 => Some(Cap::IoOutBounded),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapError {
    RatchetViolation { from: Cap, to: Cap },
    UnknownCap(u8),
}

impl fmt::Display for CapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapError::RatchetViolation { from, to } => {
                write!(f, "ratchet violation: {:?} -> {:?}", from, to)
            }
            CapError::UnknownCap(v) => write!(f, "unknown cap {}", v),
        }
    }
}

/// Poset helpers over [`Cap`].
pub struct CapPoset;

impl CapPoset {
    /// Meet (greatest lower bound).
    pub fn meet(a: Cap, b: Cap) -> Cap {
        core::cmp::min(a, b)
    }

    /// Join (least upper bound).
    pub fn join(a: Cap, b: Cap) -> Cap {
        core::cmp::max(a, b)
    }

    /// Forward-only ratchet: `to` must be ≤ `from`.
    pub fn weaken(from: Cap, to: Cap) -> Result<Cap, CapError> {
        if to <= from {
            Ok(to)
        } else {
            Err(CapError::RatchetViolation { from, to })
        }
    }
}
