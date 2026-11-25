//! SHA-256 digests for code/out/artifact ratification.

use sha2::{Digest, Sha256};

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

pub fn hex32(digest: &[u8; 32]) -> heapless_hex::Hex64 {
    heapless_hex::Hex64::from_digest(digest)
}

/// Tiny hex encoder without pulling in `hex` crate (no_std friendly).
pub mod heapless_hex {
    use core::fmt;

    pub struct Hex64 {
        buf: [u8; 64],
    }

    impl Hex64 {
        pub fn from_digest(d: &[u8; 32]) -> Self {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            let mut buf = [0u8; 64];
            for (i, b) in d.iter().enumerate() {
                buf[i * 2] = HEX[(b >> 4) as usize];
                buf[i * 2 + 1] = HEX[(b & 0xf) as usize];
            }
            Self { buf }
        }

        pub fn as_str(&self) -> &str {
            // SAFETY: only ASCII hex written above
            unsafe { core::str::from_utf8_unchecked(&self.buf) }
        }
    }

    impl fmt::Display for Hex64 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.as_str())
        }
    }
}
