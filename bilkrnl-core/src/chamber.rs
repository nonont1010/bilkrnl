//! Blast chamber: ratify results with no trust that execution was safe.

use crate::cap::Cap;
use crate::digest::sha256;
use crate::enclave::artifact_digest;
use crate::isa::{EnclaveResult, ExitReason};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutPred {
    ExactBytes(&'static [u8]),
    Sha256Eq([u8; 32]),
    LenLe(u32),
    Any,
}

#[derive(Clone, Debug)]
pub struct RatifySpec {
    pub allowed_exits: &'static [ExitReason],
    pub max_cycles: u64,
    pub max_out_bytes: u32,
    pub required_caps_ceiling: Cap,
    pub out_predicate: OutPred,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChamberReject {
    ExitNotAllowed,
    CyclesOverBudget { cycles: u64, max: u64 },
    OutTooLarge { len: u32, max: u32 },
    CapCeilingViolated { at_exit: Cap, ceiling: Cap },
    OutPredicateFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChamberDecision {
    Ratified { artifact_digest: [u8; 32] },
    Reject { reason: ChamberReject },
}

fn exit_allowed(spec: &RatifySpec, exit: ExitReason) -> bool {
    spec.allowed_exits.iter().any(|e| match (e, exit) {
        (ExitReason::Halt, ExitReason::Halt) => true,
        (ExitReason::BudgetExceeded, ExitReason::BudgetExceeded) => true,
        (ExitReason::Fault(a), ExitReason::Fault(b)) => *a == b,
        // Allow matching Fault(_) loosely if same discriminant only when equal
        _ => false,
    })
}

fn out_ok(pred: &OutPred, out: &[u8]) -> bool {
    match pred {
        OutPred::Any => true,
        OutPred::LenLe(n) => out.len() as u32 <= *n,
        OutPred::ExactBytes(exp) => out == *exp,
        OutPred::Sha256Eq(exp) => &sha256(out) == exp,
    }
}

/// Ratify an enclave result. Caller must have already disposed enclave state.
pub fn chamber_ratify(res: &EnclaveResult, expect: &RatifySpec) -> ChamberDecision {
    if !exit_allowed(expect, res.exit) {
        return ChamberDecision::Reject {
            reason: ChamberReject::ExitNotAllowed,
        };
    }
    if res.cycles > expect.max_cycles {
        return ChamberDecision::Reject {
            reason: ChamberReject::CyclesOverBudget {
                cycles: res.cycles,
                max: expect.max_cycles,
            },
        };
    }
    let out_len = res.out.len() as u32;
    if out_len > expect.max_out_bytes {
        return ChamberDecision::Reject {
            reason: ChamberReject::OutTooLarge {
                len: out_len,
                max: expect.max_out_bytes,
            },
        };
    }
    // Caps at exit must not exceed ceiling (stronger than ceiling = violation).
    if res.caps_at_exit > expect.required_caps_ceiling {
        return ChamberDecision::Reject {
            reason: ChamberReject::CapCeilingViolated {
                at_exit: res.caps_at_exit,
                ceiling: expect.required_caps_ceiling,
            },
        };
    }
    if !out_ok(&expect.out_predicate, &res.out) {
        return ChamberDecision::Reject {
            reason: ChamberReject::OutPredicateFailed,
        };
    }
    ChamberDecision::Ratified {
        artifact_digest: artifact_digest(res),
    }
}

/// Spec for the demo OK\\n program.
pub fn demo_ok_spec() -> RatifySpec {
    RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 64,
        max_out_bytes: 8,
        required_caps_ceiling: Cap::IoOutBounded,
        out_predicate: OutPred::ExactBytes(b"OK\n"),
    }
}

/// Spec for the sealed policy-gate PASS job (killer enclave workload).
///
/// Ceiling is `Compute` (not `None`): SoftISA refuses to step — including `Halt` —
/// without `Compute`, so a successful gate ends at `Compute` after dropping IO.
pub fn demo_policy_pass_spec() -> RatifySpec {
    RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 128,
        max_out_bytes: 8,
        required_caps_ceiling: Cap::Compute,
        out_predicate: OutPred::ExactBytes(b"PASS\n"),
    }
}

/// Negative twin: ratify only `FAIL\n` with the same ceilings.
pub fn demo_policy_fail_spec() -> RatifySpec {
    RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 128,
        max_out_bytes: 8,
        required_caps_ceiling: Cap::Compute,
        out_predicate: OutPred::ExactBytes(b"FAIL\n"),
    }
}

/// Spec for the sealed attested-reverse job (`BIL!` → Out `!LIB`).
pub fn demo_rev4_pass_spec() -> RatifySpec {
    RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 64,
        max_out_bytes: 8,
        required_caps_ceiling: Cap::Compute,
        out_predicate: OutPred::ExactBytes(b"!LIB"),
    }
}

/// Negative twin: ratify ExactBytes(`XXXX`) for the all-X sealed blob.
pub fn demo_rev4_fail_spec() -> RatifySpec {
    RatifySpec {
        allowed_exits: &[ExitReason::Halt],
        max_cycles: 64,
        max_out_bytes: 8,
        required_caps_ceiling: Cap::Compute,
        out_predicate: OutPred::ExactBytes(b"XXXX"),
    }
}
