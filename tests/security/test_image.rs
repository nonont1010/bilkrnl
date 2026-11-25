//! TC-06: bad magic, truncated image, entry OOB.

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::image::{BytecodeImage, ImageError, IMAGE_MAGIC};
use bilkrnl_core::isa::Op;

#[test]
fn tc06_bad_magic() {
    let mut bytes = BytecodeImage::new(
        0,
        vec![Op::Halt as u8],
        Cap::Compute,
        ResourceBudget::new(8, 0, 1),
    )
    .unwrap()
    .encode();
    bytes[0] = b'X';
    match BytecodeImage::decode(&bytes) {
        Err(ImageError::BadMagic(m)) => assert_ne!(m, IMAGE_MAGIC),
        other => panic!("{:?}", other),
    }
}

#[test]
fn tc06_truncated() {
    let bytes = BytecodeImage::new(
        0,
        vec![Op::Halt as u8, Op::Nop as u8, Op::Nop as u8],
        Cap::Compute,
        ResourceBudget::new(8, 0, 1),
    )
    .unwrap()
    .encode();
    let short = &bytes[..bytes.len() - 1];
    assert!(matches!(
        BytecodeImage::decode(short),
        Err(ImageError::Truncated { .. })
    ));
}

#[test]
fn tc06_entry_oob() {
    match BytecodeImage::new(
        5,
        vec![Op::Halt as u8],
        Cap::Compute,
        ResourceBudget::new(8, 0, 1),
    ) {
        Err(ImageError::EntryOob { .. }) => {}
        other => panic!("{:?}", other),
    }
}

#[test]
fn tc06_too_short() {
    assert_eq!(BytecodeImage::decode(&[0u8; 8]), Err(ImageError::TooShort));
}
