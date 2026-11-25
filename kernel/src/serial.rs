//! Serial output via UEFI console stdout (works on QEMU virt).

use core::fmt::Write;
use uefi::system;

pub fn write_str(s: &str) {
    let _ = system::with_stdout(|out| out.write_str(s));
}
