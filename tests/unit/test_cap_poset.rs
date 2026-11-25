//! TC-09: capability ratchet — weaken only.

use bilkrnl_core::cap::{Cap, CapError, CapPoset};

#[test]
fn tc09_weaken_compute_to_none_ok() {
    assert_eq!(CapPoset::weaken(Cap::Compute, Cap::None).unwrap(), Cap::None);
}

#[test]
fn tc09_weaken_io_to_compute_ok() {
    assert_eq!(
        CapPoset::weaken(Cap::IoOutBounded, Cap::Compute).unwrap(),
        Cap::Compute
    );
}

#[test]
fn tc09_strengthen_none_to_compute_fails() {
    match CapPoset::weaken(Cap::None, Cap::Compute) {
        Err(CapError::RatchetViolation { from, to }) => {
            assert_eq!(from, Cap::None);
            assert_eq!(to, Cap::Compute);
        }
        other => panic!("expected RatchetViolation, got {:?}", other),
    }
}

#[test]
fn tc09_meet_join() {
    assert_eq!(CapPoset::meet(Cap::IoOutBounded, Cap::Compute), Cap::Compute);
    assert_eq!(CapPoset::join(Cap::None, Cap::Compute), Cap::Compute);
}
