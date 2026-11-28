//! Lower bil stack-bytecode (BIL1 profile subset) to SoftISA `BytecodeImage` bytes.

use std::collections::BTreeSet;
use std::fmt;

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::image::BytecodeImage;

use crate::stack_bc::{parse_stack_bc, StackFn, StackOp, StackProgram};
use crate::Assembled;

const INTRINSIC_OUT: &str = "bil1_out";
const INTRINSIC_WEAKEN: &str = "bil1_weaken";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LowerError {
    pub message: String,
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LowerError {}

impl LowerError {
    fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

/// Parse stack-bytecode text and lower the profile entry function to a BIL1 image.
pub fn lower_stack_bc(source: &str) -> Result<Assembled, LowerError> {
    let prog = parse_stack_bc(source).map_err(|e| LowerError::new(e.to_string()))?;
    lower_program(&prog)
}

/// Lower an already-parsed [`StackProgram`].
pub fn lower_program(prog: &StackProgram) -> Result<Assembled, LowerError> {
    let func = select_entry(prog)?;
    lower_function(prog, func)
}

fn select_entry<'a>(prog: &'a StackProgram) -> Result<&'a StackFn, LowerError> {
    if let Some(name) = &prog.entry_fn {
        return prog
            .functions
            .iter()
            .find(|f| f.name == *name)
            .ok_or_else(|| LowerError::new(format!("entry function '{name}' not found")));
    }

    let candidates: Vec<&StackFn> = prog
        .functions
        .iter()
        .filter(|f| !is_intrinsic_name(&f.name) && !f.name.starts_with("__global_"))
        .collect();

    match candidates.len() {
        0 => Err(LowerError::new("no lowerable entry function found")),
        1 => Ok(candidates[0]),
        _ => {
            // Prefer conventional names, else reject ambiguity.
            for prefer in ["emit", "main", "prog", "start"] {
                if let Some(f) = candidates.iter().find(|f| f.name == prefer) {
                    return Ok(*f);
                }
            }
            Err(LowerError::new(format!(
                "ambiguous entry function; set `.fn <name>` (candidates: {})",
                candidates
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
        }
    }
}

fn is_intrinsic_name(name: &str) -> bool {
    name == INTRINSIC_OUT || name == INTRINSIC_WEAKEN
}

#[derive(Clone, Debug)]
enum SoftOp {
    LoadImm(u8),
    AddImm(u8),
    SubImm(u8),
    Out,
    Weaken(u8),
    Halt,
    /// Resolved against var slot table after `code_len` is known.
    LoadVar(String),
    StoreVar(String),
}

#[derive(Clone, Debug)]
enum V {
    Lit(u8),
    NeedLoad(String),
    InAcc,
}

fn lower_function(prog: &StackProgram, func: &StackFn) -> Result<Assembled, LowerError> {
    let mut soft: Vec<SoftOp> = Vec::new();
    let mut stack: Vec<V> = Vec::new();
    let mut vars: BTreeSet<String> = BTreeSet::new();
    let mut uses_out = false;

    for op in &func.ops {
        match op {
            StackOp::PushInt(v) => {
                let k = check_u8_int(*v)?;
                stack.push(V::Lit(k));
            }
            StackOp::PushFloat(_) => {
                return Err(LowerError::new(
                    "reject: floats are not in the BIL1 profile (PushFloat)",
                ));
            }
            StackOp::PushStr(_) => {
                return Err(LowerError::new(
                    "reject: strings are not in the BIL1 profile (PushStr)",
                ));
            }
            StackOp::PushBool(_) => {
                return Err(LowerError::new(
                    "reject: bools are not in the BIL1 profile (PushBool)",
                ));
            }
            StackOp::LoadVar(name) => {
                vars.insert(name.clone());
                stack.push(V::NeedLoad(name.clone()));
            }
            StackOp::StoreVar(name) => {
                vars.insert(name.clone());
                let top = pop(&mut stack, "StoreVar")?;
                materialize(&mut soft, top)?;
                soft.push(SoftOp::StoreVar(name.clone()));
                stack.push(V::InAcc);
            }
            StackOp::PopVar(name) => {
                // Env cleanup only; SoftISA slots persist.
                vars.insert(name.clone());
            }
            StackOp::Binary(opname) => match opname.as_str() {
                "Mul" | "Div" | "Mod" => {
                    return Err(LowerError::new(format!(
                        "reject: Binary({opname}) is not in the BIL1 profile"
                    )));
                }
                "Add" | "Sub" => {
                    let rhs = pop(&mut stack, "Binary")?;
                    let lhs = pop(&mut stack, "Binary")?;
                    lower_add_sub(&mut soft, opname, lhs, rhs)?;
                    stack.push(V::InAcc);
                }
                other => {
                    return Err(LowerError::new(format!(
                        "reject: Binary({other}) is not in the BIL1 profile"
                    )));
                }
            },
            StackOp::Unary(opname) => {
                return Err(LowerError::new(format!(
                    "reject: Unary({opname}) is not in the BIL1 profile"
                )));
            }
            StackOp::Call { func: fname, argc } => {
                if *argc != 1 {
                    return Err(LowerError::new(format!(
                        "reject: Call({fname}) argc={argc}; profile intrinsics require argc=1"
                    )));
                }
                let arg = pop(&mut stack, "Call")?;
                if fname == INTRINSIC_OUT {
                    materialize(&mut soft, arg)?;
                    soft.push(SoftOp::Out);
                    uses_out = true;
                    stack.push(V::InAcc);
                } else if fname == INTRINSIC_WEAKEN {
                    let lit = match arg {
                        V::Lit(c) => c,
                        other => {
                            return Err(LowerError::new(format!(
                                "reject: bil1_weaken requires a literal cap 0..=2, got {other:?}"
                            )));
                        }
                    };
                    if lit > 2 {
                        return Err(LowerError::new(format!(
                            "reject: bil1_weaken cap {lit} outside 0..=2"
                        )));
                    }
                    soft.push(SoftOp::Weaken(lit));
                    stack.push(V::Lit(lit));
                } else {
                    return Err(LowerError::new(format!(
                        "reject: Call(\"{fname}\") — only bil1_out / bil1_weaken are allowed"
                    )));
                }
            }
            StackOp::Return => {
                // Discard unused return value / leftovers; SoftISA ends at Halt.
                stack.clear();
                soft.push(SoftOp::Halt);
            }
            StackOp::MakeList(_) => {
                return Err(LowerError::new(
                    "reject: lists are not in the BIL1 profile (MakeList)",
                ));
            }
            StackOp::MakeStruct { .. } | StackOp::UpdateStruct { .. } | StackOp::GetField(_) => {
                return Err(LowerError::new(
                    "reject: structs are not in the BIL1 profile",
                ));
            }
            StackOp::Match => {
                return Err(LowerError::new(
                    "reject: Match is not in the BIL1 profile",
                ));
            }
            StackOp::Quantifier => {
                return Err(LowerError::new(
                    "reject: Quantifier is not in the BIL1 profile",
                ));
            }
            StackOp::GuardedSelect => {
                return Err(LowerError::new(
                    "reject: nondeterministic / GuardedSelect not lowered in B3.2 profile subset",
                ));
            }
            StackOp::GuardedLoop => {
                return Err(LowerError::new(
                    "reject: GuardedLoop not lowered in B3.2 profile subset (no variant-driven form yet)",
                ));
            }
            StackOp::PipeStage(_) => {
                return Err(LowerError::new(
                    "reject: PipeStage is not in the BIL1 profile",
                ));
            }
            StackOp::Unknown(s) if s == "TrapAbort" || s.contains("TrapAbort") => {
                return Err(LowerError::new(
                    "reject: TrapAbort — never emit host-escape opcodes 0xE0..=0xEF",
                ));
            }
            StackOp::Unknown(s) => {
                return Err(LowerError::new(format!(
                    "reject: unsupported stack op '{s}'"
                )));
            }
        }
    }

    if soft.is_empty() || !matches!(soft.last(), Some(SoftOp::Halt)) {
        // Implicit halt if the dump omitted Return (shouldn't happen for bil).
        soft.push(SoftOp::Halt);
    }

    // Encode with slot addresses after code_len.
    let code = encode_soft(&soft, &vars)?;

    let caps = resolve_caps(prog, uses_out)?;
    let cycles = prog.budget_cycles.unwrap_or(64);
    let out_b = prog.budget_out.unwrap_or(8);
    let pages = prog.budget_pages.unwrap_or(1);

    let image = BytecodeImage::new(
        0,
        code,
        caps,
        ResourceBudget::new(cycles, out_b, pages),
    )
    .map_err(|e| LowerError::new(format!("image: {e:?}")))?;

    Ok(Assembled { image })
}

fn check_u8_int(v: i64) -> Result<u8, LowerError> {
    if (0..=255).contains(&v) {
        Ok(v as u8)
    } else {
        Err(LowerError::new(format!(
            "reject: Int {v} outside 0..=255"
        )))
    }
}

fn pop(stack: &mut Vec<V>, ctx: &str) -> Result<V, LowerError> {
    stack
        .pop()
        .ok_or_else(|| LowerError::new(format!("reject: value stack underflow at {ctx}")))
}

fn materialize(soft: &mut Vec<SoftOp>, v: V) -> Result<(), LowerError> {
    match v {
        V::Lit(k) => soft.push(SoftOp::LoadImm(k)),
        V::NeedLoad(name) => soft.push(SoftOp::LoadVar(name)),
        V::InAcc => {}
    }
    Ok(())
}

fn lower_add_sub(
    soft: &mut Vec<SoftOp>,
    opname: &str,
    lhs: V,
    rhs: V,
) -> Result<(), LowerError> {
    match (opname, lhs, rhs) {
        ("Add", V::Lit(a), V::Lit(b)) => {
            soft.push(SoftOp::LoadImm(a));
            soft.push(SoftOp::AddImm(b));
        }
        ("Add", V::NeedLoad(n), V::Lit(k)) | ("Add", V::Lit(k), V::NeedLoad(n)) => {
            soft.push(SoftOp::LoadVar(n));
            soft.push(SoftOp::AddImm(k));
        }
        ("Add", V::InAcc, V::Lit(k)) | ("Add", V::Lit(k), V::InAcc) => {
            // If Lit was second and InAcc first, AddImm works. If Lit first, Acc holds other —
            // profile requires one operand literal; InAcc + Lit is fine as AddImm.
            soft.push(SoftOp::AddImm(k));
        }
        ("Add", V::NeedLoad(_), V::NeedLoad(_))
        | ("Add", V::NeedLoad(_), V::InAcc)
        | ("Add", V::InAcc, V::NeedLoad(_))
        | ("Add", V::InAcc, V::InAcc) => {
            return Err(LowerError::new(
                "reject: var+var arithmetic (Binary(Add) needs one literal PushInt)",
            ));
        }
        ("Sub", V::Lit(a), V::Lit(b)) => {
            soft.push(SoftOp::LoadImm(a));
            soft.push(SoftOp::SubImm(b));
        }
        ("Sub", V::NeedLoad(n), V::Lit(k)) => {
            soft.push(SoftOp::LoadVar(n));
            soft.push(SoftOp::SubImm(k));
        }
        ("Sub", V::InAcc, V::Lit(k)) => {
            soft.push(SoftOp::SubImm(k));
        }
        ("Sub", V::Lit(_), V::NeedLoad(_)) | ("Sub", V::Lit(_), V::InAcc) => {
            return Err(LowerError::new(
                "reject: Binary(Sub) requires literal on the right (x - k), not k - x",
            ));
        }
        ("Sub", V::NeedLoad(_), V::NeedLoad(_))
        | ("Sub", V::NeedLoad(_), V::InAcc)
        | ("Sub", V::InAcc, V::NeedLoad(_))
        | ("Sub", V::InAcc, V::InAcc) => {
            return Err(LowerError::new(
                "reject: var+var arithmetic (Binary(Sub) needs one literal PushInt)",
            ));
        }
        (other, _, _) => {
            return Err(LowerError::new(format!(
                "reject: cannot lower Binary({other})"
            )));
        }
    }
    Ok(())
}

fn encode_soft(soft: &[SoftOp], vars: &BTreeSet<String>) -> Result<Vec<u8>, LowerError> {
    // Pass 1: size without resolving var addresses.
    let mut code_len = 0usize;
    for op in soft {
        code_len += soft_size(op);
    }
    if code_len > 64 * 1024 {
        return Err(LowerError::new(format!("code too large: {code_len}")));
    }

    // Sorted var names → ascending u16 addresses after code.
    let mut slot_map: Vec<(String, u16)> = Vec::new();
    let mut addr = code_len as u16;
    for name in vars {
        // Skip if never actually Load/Store'd into code — still assign for PopVar-only.
        slot_map.push((name.clone(), addr));
        addr = addr.checked_add(1).ok_or_else(|| {
            LowerError::new("reject: variable slot address overflow")
        })?;
    }

    let lookup = |name: &str| -> Result<u16, LowerError> {
        slot_map
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, a)| *a)
            .ok_or_else(|| LowerError::new(format!("internal: missing slot for '{name}'")))
    };

    let mut code = Vec::with_capacity(code_len);
    for op in soft {
        match op {
            SoftOp::LoadImm(k) => {
                code.push(0x02);
                code.push(*k);
            }
            SoftOp::LoadVar(n) => {
                let a = lookup(n)?;
                code.push(0x03);
                code.extend_from_slice(&a.to_le_bytes());
            }
            SoftOp::StoreVar(n) => {
                let a = lookup(n)?;
                code.push(0x04);
                code.extend_from_slice(&a.to_le_bytes());
            }
            SoftOp::AddImm(k) => {
                code.push(0x05);
                code.push(*k);
            }
            SoftOp::SubImm(k) => {
                code.push(0x06);
                code.push(*k);
            }
            SoftOp::Out => code.push(0x09),
            SoftOp::Weaken(c) => {
                code.push(0x0A);
                code.push(*c);
            }
            SoftOp::Halt => code.push(0x00),
        }
        // Never emit host-escape opcodes.
        let opb = code[code.len() - soft_size(op)];
        if (0xE0..=0xEF).contains(&opb) {
            return Err(LowerError::new(format!(
                "reject: refusing host-escape opcode {opb:#04x}"
            )));
        }
    }

    if code.len() != code_len {
        return Err(LowerError::new(format!(
            "internal size mismatch: expected {code_len}, got {}",
            code.len()
        )));
    }
    Ok(code)
}

fn soft_size(op: &SoftOp) -> usize {
    match op {
        SoftOp::Halt | SoftOp::Out => 1,
        SoftOp::LoadImm(_) | SoftOp::AddImm(_) | SoftOp::SubImm(_) | SoftOp::Weaken(_) => 2,
        SoftOp::LoadVar(_) | SoftOp::StoreVar(_) => 3,
    }
}

fn resolve_caps(prog: &StackProgram, uses_out: bool) -> Result<Cap, LowerError> {
    if let Some(c) = &prog.caps {
        return parse_cap(c);
    }
    Ok(if uses_out {
        Cap::IoOutBounded
    } else {
        Cap::Compute
    })
}

fn parse_cap(s: &str) -> Result<Cap, LowerError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "none" | "0" => Ok(Cap::None),
        "compute" | "1" => Ok(Cap::Compute),
        "iooutbounded" | "io" | "2" => Ok(Cap::IoOutBounded),
        other => Err(LowerError::new(format!(
            "unknown cap '{other}' (None|Compute|IoOutBounded)"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bilkrnl_core::isa::{ExitReason, Vm};

    const OK_BC: &str = r#"
.caps IoOutBounded
.budget cycles=64 out=8 pages=1
.fn emit
0000: PushInt(79)
0001: Call { func: "bil1_out", argc: 1 }
0002: PushInt(75)
0003: Call { func: "bil1_out", argc: 1 }
0004: PushInt(10)
0005: Call { func: "bil1_out", argc: 1 }
0006: Return
"#;

    #[test]
    fn lower_examples_out_hello_bc_txt() {
        let src = include_str!("../examples/lower/out_hello.bc.txt");
        let a = lower_stack_bc(src).expect("lower out_hello.bc.txt");
        assert_eq!(
            a.image.code,
            vec![0x02, 0x4f, 0x09, 0x02, 0x4b, 0x09, 0x02, 0x0a, 0x09, 0x00]
        );
    }

    #[test]
    fn lower_ok_matches_fixture_bytes() {
        let a = lower_stack_bc(OK_BC).expect("lower");
        assert_eq!(
            a.image.code,
            vec![0x02, 0x4f, 0x09, 0x02, 0x4b, 0x09, 0x02, 0x0a, 0x09, 0x00]
        );
        let bytes = a.image.encode();
        let expect: [u8; 42] = [
            0x42, 0x49, 0x4c, 0x31, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x02, 0x00,
            0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x02, 0x4f, 0x09, 0x02, 0x4b, 0x09, 0x02, 0x0a, 0x09, 0x00,
        ];
        assert_eq!(bytes, expect);

        let mut vm = Vm::new(
            &a.image.code,
            a.image.entry,
            a.image.budget,
            a.image.initial_caps,
        )
        .unwrap();
        let r = vm.run();
        assert_eq!(r.exit, ExitReason::Halt);
        assert_eq!(r.out, b"OK\n");
    }

    #[test]
    fn lower_add_imm_with_slots() {
        let src = r#"
.fn prog
0000: PushInt(10)
0001: StoreVar("a")
0002: LoadVar("a")
0003: PushInt(5)
0004: Binary(Add)
0005: PopVar("a")
0006: Return
"#;
        let a = lower_stack_bc(src).unwrap();
        // ldi 10; st @code_len; ld @code_len; addi 5; halt
        // code_len = 2+3+3+2+1 = 11, slot a @ 11
        assert_eq!(
            a.image.code,
            vec![
                0x02, 10, // ldi
                0x04, 11, 0, // st @11
                0x03, 11, 0, // ld @11
                0x05, 5, // addi 5
                0x00, // halt
            ]
        );
        assert_eq!(a.image.initial_caps, Cap::Compute);
    }

    #[test]
    fn reject_mul() {
        let err = lower_stack_bc("PushInt(1)\nPushInt(2)\nBinary(Mul)\nReturn\n").unwrap_err();
        assert!(err.message.contains("Binary(Mul)"), "{err}");
    }

    #[test]
    fn reject_var_var_add() {
        let src = r#"
PushInt(1)
StoreVar("a")
PushInt(2)
StoreVar("b")
LoadVar("a")
LoadVar("b")
Binary(Add)
Return
"#;
        let err = lower_stack_bc(src).unwrap_err();
        assert!(
            err.message.contains("var+var") || err.message.contains("literal"),
            "{err}"
        );
    }

    #[test]
    fn reject_foreign_call() {
        let err = lower_stack_bc(
            "PushInt(1)\nCall { func: \"helper\", argc: 1 }\nReturn\n",
        )
        .unwrap_err();
        assert!(err.message.contains("bil1_out"), "{err}");
        assert!(err.message.contains("helper"), "{err}");
    }

    #[test]
    fn reject_int_out_of_range() {
        let err = lower_stack_bc("PushInt(256)\nReturn\n").unwrap_err();
        assert!(err.message.contains("0..=255"), "{err}");
    }

    #[test]
    fn reject_float() {
        let err = lower_stack_bc("PushFloat(1.5)\nReturn\n").unwrap_err();
        assert!(err.message.contains("float"), "{err}");
    }

    #[test]
    fn reject_trap_abort() {
        let err = lower_stack_bc("TrapAbort\n").unwrap_err();
        assert!(err.message.contains("TrapAbort"), "{err}");
    }
}
