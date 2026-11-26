//! BIL1 profile IR assembler (B3.1).
//!
//! Host-only. Assembles hand-written SoftISA IR into `BytecodeImage` bytes.
//! Never linked into the UEFI kernel.

use std::collections::BTreeMap;
use std::fmt;

use bilkrnl_core::budget::ResourceBudget;
use bilkrnl_core::cap::Cap;
use bilkrnl_core::image::BytecodeImage;

/// Parsed + assembled image ready to encode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assembled {
    pub image: BytecodeImage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AsmError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for AsmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for AsmError {}

#[derive(Clone, Debug)]
enum ImmOrLabel {
    Imm(u16),
    Label(String),
}

#[derive(Clone, Debug)]
enum Item {
    Byte(Vec<u8>),
    Op { opcode: u8, args: Vec<ImmOrLabel>, size: usize },
    Label(String),
}

#[derive(Clone, Debug)]
struct Header {
    caps: Cap,
    max_cycles: u64,
    max_out_bytes: u32,
    max_mem_pages: u16,
    entry: Option<String>,
}

/// Assemble profile IR text into a `BytecodeImage`.
pub fn assemble(source: &str) -> Result<Assembled, AsmError> {
    let (header, items) = parse(source)?;
    let (code, entry) = encode(&header, &items)?;
    let image = BytecodeImage::new(
        entry,
        code,
        header.caps,
        ResourceBudget::new(header.max_cycles, header.max_out_bytes, header.max_mem_pages),
    )
    .map_err(|e| AsmError {
        line: 0,
        message: format!("image: {e:?}"),
    })?;
    Ok(Assembled { image })
}

fn parse(source: &str) -> Result<(Header, Vec<Item>), AsmError> {
    let mut header = Header {
        caps: Cap::Compute,
        max_cycles: 64,
        max_out_bytes: 8,
        max_mem_pages: 1,
        entry: None,
    };
    let mut items = Vec::new();
    let mut saw_caps = false;
    let mut saw_budget = false;

    for (idx, raw) in source.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix('.') {
            let (dir, args) = split_first(rest);
            match dir {
                "caps" => {
                    header.caps = parse_cap(args, line_no)?;
                    saw_caps = true;
                }
                "budget" => {
                    parse_budget(args, &mut header, line_no)?;
                    saw_budget = true;
                }
                "entry" => {
                    let name = args.trim();
                    if name.is_empty() || name.contains(char::is_whitespace) {
                        return Err(AsmError {
                            line: line_no,
                            message: "expected `.entry <label>`".into(),
                        });
                    }
                    header.entry = Some(name.to_string());
                }
                "byte" => {
                    items.push(Item::Byte(parse_bytes(args, line_no)?));
                }
                other => {
                    return Err(AsmError {
                        line: line_no,
                        message: format!("unknown directive '.{other}'"),
                    });
                }
            }
            continue;
        }

        if let Some(name) = line.strip_suffix(':') {
            let name = name.trim();
            if !is_ident(name) {
                return Err(AsmError {
                    line: line_no,
                    message: format!("invalid label '{name}'"),
                });
            }
            items.push(Item::Label(name.to_string()));
            continue;
        }

        let (mnem, args) = split_first(line);
        items.push(parse_op(mnem, args, line_no)?);
    }

    if !saw_caps {
        return Err(AsmError {
            line: 0,
            message: "missing `.caps` directive".into(),
        });
    }
    if !saw_budget {
        return Err(AsmError {
            line: 0,
            message: "missing `.budget` directive".into(),
        });
    }
    if header.entry.is_none() {
        return Err(AsmError {
            line: 0,
            message: "missing `.entry` directive".into(),
        });
    }

    Ok((header, items))
}

fn encode(header: &Header, items: &[Item]) -> Result<(Vec<u8>, u32), AsmError> {
    // Pass 1: assign addresses to labels.
    let mut labels: BTreeMap<String, u16> = BTreeMap::new();
    let mut pc: usize = 0;
    for item in items {
        match item {
            Item::Label(name) => {
                if labels.insert(name.clone(), pc as u16).is_some() {
                    return Err(AsmError {
                        line: 0,
                        message: format!("duplicate label '{name}'"),
                    });
                }
            }
            Item::Byte(b) => pc += b.len(),
            Item::Op { size, .. } => pc += size,
        }
    }
    if pc > 64 * 1024 {
        return Err(AsmError {
            line: 0,
            message: format!("code too large: {pc} bytes"),
        });
    }

    let entry_name = header.entry.as_ref().unwrap();
    let entry = *labels.get(entry_name).ok_or_else(|| AsmError {
        line: 0,
        message: format!("entry label '{entry_name}' not found"),
    })? as u32;

    // Pass 2: emit bytes.
    let mut code = Vec::with_capacity(pc);
    for item in items {
        match item {
            Item::Label(_) => {}
            Item::Byte(b) => code.extend_from_slice(b),
            Item::Op { opcode, args, size } => {
                let start = code.len();
                code.push(*opcode);
                for arg in args {
                    let v = resolve(arg, &labels)?;
                    match expected_width(*opcode, args.len(), code.len() - start - 1) {
                        1 => {
                            if v > 0xff {
                                return Err(AsmError {
                                    line: 0,
                                    message: format!("imm8 out of range: {v}"),
                                });
                            }
                            code.push(v as u8);
                        }
                        2 => {
                            code.push((v & 0xff) as u8);
                            code.push((v >> 8) as u8);
                        }
                        _ => unreachable!(),
                    }
                }
                if code.len() - start != *size {
                    return Err(AsmError {
                        line: 0,
                        message: format!("internal size mismatch for opcode {opcode:#04x}"),
                    });
                }
                // Refuse host-escape opcodes in opcode position.
                if (0xE0..=0xEF).contains(opcode) {
                    return Err(AsmError {
                        line: 0,
                        message: format!("refusing host-escape opcode {opcode:#04x}"),
                    });
                }
            }
        }
    }

    // Jump targets must land inside code (Vm also checks mem bounds).
    Ok((code, entry))
}

fn resolve(arg: &ImmOrLabel, labels: &BTreeMap<String, u16>) -> Result<u16, AsmError> {
    match arg {
        ImmOrLabel::Imm(v) => Ok(*v),
        ImmOrLabel::Label(name) => labels.get(name).copied().ok_or_else(|| AsmError {
            line: 0,
            message: format!("undefined label '{name}'"),
        }),
    }
}

fn expected_width(opcode: u8, argc: usize, arg_index: usize) -> usize {
    let _ = (argc, arg_index);
    match opcode {
        0x07 | 0x08 => 2, // jmp/jz: u16
        0x03 | 0x04 => 2, // ld/st: u16 addr
        _ => 1,
    }
}

fn parse_op(mnem: &str, args: &str, line: usize) -> Result<Item, AsmError> {
    let mnem = mnem.to_ascii_lowercase();
    let (opcode, kind) = match mnem.as_str() {
        "halt" => (0x00u8, OpKind::None),
        "nop" => (0x01, OpKind::None),
        "ldi" => (0x02, OpKind::Imm8),
        "ld" => (0x03, OpKind::Addr),
        "st" => (0x04, OpKind::Addr),
        "addi" => (0x05, OpKind::Imm8),
        "subi" => (0x06, OpKind::Imm8),
        "jmp" => (0x07, OpKind::LabelOrAddr),
        "jz" => (0x08, OpKind::LabelOrAddr),
        "out" => (0x09, OpKind::None),
        "weaken" => (0x0A, OpKind::Cap),
        other => {
            return Err(AsmError {
                line,
                message: format!("unknown mnemonic '{other}'"),
            });
        }
    };

    let args = args.trim();
    let parsed = match kind {
        OpKind::None => {
            if !args.is_empty() {
                return Err(AsmError {
                    line,
                    message: format!("'{mnem}' takes no operands"),
                });
            }
            vec![]
        }
        OpKind::Imm8 => {
            let v = parse_u16_token(args, line)?;
            if v > 0xff {
                return Err(AsmError {
                    line,
                    message: format!("imm8 out of range: {v}"),
                });
            }
            vec![ImmOrLabel::Imm(v)]
        }
        OpKind::Addr => {
            vec![parse_addr_or_label(args, line)?]
        }
        OpKind::LabelOrAddr => {
            vec![parse_addr_or_label(args, line)?]
        }
        OpKind::Cap => {
            let c = parse_cap(args, line)?;
            vec![ImmOrLabel::Imm(c.as_u8() as u16)]
        }
    };

    let size = 1 + parsed
        .iter()
        .map(|a| match a {
            ImmOrLabel::Imm(_) | ImmOrLabel::Label(_) => match kind {
                OpKind::Addr | OpKind::LabelOrAddr => 2,
                _ => 1,
            },
        })
        .sum::<usize>();

    Ok(Item::Op {
        opcode,
        args: parsed,
        size,
    })
}

enum OpKind {
    None,
    Imm8,
    Addr,
    LabelOrAddr,
    Cap,
}

fn parse_addr_or_label(tok: &str, line: usize) -> Result<ImmOrLabel, AsmError> {
    let tok = tok.trim();
    if tok.is_empty() {
        return Err(AsmError {
            line,
            message: "expected address or label".into(),
        });
    }
    if let Some(rest) = tok.strip_prefix('@') {
        return Ok(ImmOrLabel::Imm(parse_u16_token(rest, line)?));
    }
    if tok.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return Ok(ImmOrLabel::Imm(parse_u16_token(tok, line)?));
    }
    if is_ident(tok) {
        return Ok(ImmOrLabel::Label(tok.to_string()));
    }
    Err(AsmError {
        line,
        message: format!("invalid address/label '{tok}'"),
    })
}

fn parse_bytes(args: &str, line: usize) -> Result<Vec<u8>, AsmError> {
    let mut out = Vec::new();
    for tok in args.split(|c: char| c == ',' || c.is_whitespace()) {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        let v = parse_u16_token(tok, line)?;
        if v > 0xff {
            return Err(AsmError {
                line,
                message: format!("byte out of range: {v}"),
            });
        }
        out.push(v as u8);
    }
    if out.is_empty() {
        return Err(AsmError {
            line,
            message: "`.byte` needs at least one value".into(),
        });
    }
    Ok(out)
}

fn parse_budget(args: &str, header: &mut Header, line: usize) -> Result<(), AsmError> {
    // cycles=N out=N pages=N
    for tok in args.split_whitespace() {
        let Some((k, v)) = tok.split_once('=') else {
            return Err(AsmError {
                line,
                message: format!("expected key=value in .budget, got '{tok}'"),
            });
        };
        match k {
            "cycles" => header.max_cycles = parse_u64_token(v, line)?,
            "out" => {
                let n = parse_u64_token(v, line)?;
                if n > u32::MAX as u64 {
                    return Err(AsmError {
                        line,
                        message: format!("out too large: {n}"),
                    });
                }
                header.max_out_bytes = n as u32;
            }
            "pages" => {
                let n = parse_u64_token(v, line)?;
                if n == 0 || n > u16::MAX as u64 {
                    return Err(AsmError {
                        line,
                        message: "pages must be 1..=65535".into(),
                    });
                }
                header.max_mem_pages = n as u16;
            }
            other => {
                return Err(AsmError {
                    line,
                    message: format!("unknown budget key '{other}'"),
                });
            }
        }
    }
    Ok(())
}

fn parse_cap(s: &str, line: usize) -> Result<Cap, AsmError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "none" | "0" => Ok(Cap::None),
        "compute" | "1" => Ok(Cap::Compute),
        "iooutbounded" | "io" | "2" => Ok(Cap::IoOutBounded),
        other => Err(AsmError {
            line,
            message: format!("unknown cap '{other}' (None|Compute|IoOutBounded)"),
        }),
    }
}

fn parse_u16_token(s: &str, line: usize) -> Result<u16, AsmError> {
    let v = parse_u64_token(s, line)?;
    u16::try_from(v).map_err(|_| AsmError {
        line,
        message: format!("integer out of u16 range: {v}"),
    })
}

fn parse_u64_token(s: &str, line: usize) -> Result<u64, AsmError> {
    let s = s.trim();
    let (radix, digits) = if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (16, h)
    } else {
        (10, s)
    };
    u64::from_str_radix(digits, radix).map_err(|_| AsmError {
        line,
        message: format!("invalid integer '{s}'"),
    })
}

fn strip_comment(line: &str) -> &str {
    match line.find(';') {
        Some(i) => &line[..i],
        None => line,
    }
}

fn split_first(s: &str) -> (&str, &str) {
    let s = s.trim();
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i + 1..].trim()),
        None => (s, ""),
    }
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Disassemble a `BytecodeImage` to profile IR (best-effort; labels as @addr).
pub fn disassemble(img: &BytecodeImage) -> String {
    let mut out = String::new();
    out.push_str(&format!(".caps   {}\n", cap_name(img.initial_caps)));
    out.push_str(&format!(
        ".budget cycles={} out={} pages={}\n",
        img.budget.max_cycles, img.budget.max_out_bytes, img.budget.max_mem_pages
    ));
    out.push_str(&format!(".entry  @{}\n", img.entry));
    if img.entry > 0 {
        out.push_str("; data prefix\n");
        for (i, b) in img.code[..img.entry as usize].iter().enumerate() {
            out.push_str(&format!(".byte 0x{b:02x}  ; @{i}\n"));
        }
    }
    out.push_str("start:\n");
    let mut pc = img.entry as usize;
    while pc < img.code.len() {
        let op = img.code[pc];
        let (text, size) = match op {
            0x00 => ("    halt".to_string(), 1),
            0x01 => ("    nop".to_string(), 1),
            0x02 if pc + 1 < img.code.len() => {
                (format!("    ldi 0x{:02x}", img.code[pc + 1]), 2)
            }
            0x03 if pc + 2 < img.code.len() => {
                let a = u16::from_le_bytes([img.code[pc + 1], img.code[pc + 2]]);
                (format!("    ld @{a}"), 3)
            }
            0x04 if pc + 2 < img.code.len() => {
                let a = u16::from_le_bytes([img.code[pc + 1], img.code[pc + 2]]);
                (format!("    st @{a}"), 3)
            }
            0x05 if pc + 1 < img.code.len() => {
                (format!("    addi 0x{:02x}", img.code[pc + 1]), 2)
            }
            0x06 if pc + 1 < img.code.len() => {
                (format!("    subi 0x{:02x}", img.code[pc + 1]), 2)
            }
            0x07 if pc + 2 < img.code.len() => {
                let a = u16::from_le_bytes([img.code[pc + 1], img.code[pc + 2]]);
                (format!("    jmp @{a}"), 3)
            }
            0x08 if pc + 2 < img.code.len() => {
                let a = u16::from_le_bytes([img.code[pc + 1], img.code[pc + 2]]);
                (format!("    jz @{a}"), 3)
            }
            0x09 => ("    out".to_string(), 1),
            0x0A if pc + 1 < img.code.len() => {
                let c = Cap::from_u8(img.code[pc + 1])
                    .map(cap_name)
                    .unwrap_or("?");
                (format!("    weaken {c}"), 2)
            }
            other => (format!("    ; raw 0x{other:02x}"), 1),
        };
        out.push_str(&text);
        out.push('\n');
        pc += size;
    }
    out
}

fn cap_name(c: Cap) -> &'static str {
    match c {
        Cap::None => "None",
        Cap::Compute => "Compute",
        Cap::IoOutBounded => "IoOutBounded",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bilkrnl_core::digest::sha256;
    use bilkrnl_core::isa::{ExitReason, Vm};

    const OK_IR: &str = r#"
.caps   IoOutBounded
.budget cycles=64 out=8 pages=1
.entry  start
start:
    ldi 0x4f
    out
    ldi 0x4b
    out
    ldi 0x0a
    out
    halt
"#;

    #[test]
    fn assemble_ok_matches_golden_bytes() {
        let a = assemble(OK_IR).expect("assemble");
        let bytes = a.image.encode();
        // Header + 10 code bytes from tools/bil1asm/examples/ok.bil1.txt
        let expect: [u8; 42] = [
            0x42, 0x49, 0x4c, 0x31, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x02, 0x00,
            0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x02, 0x4f, 0x09, 0x02, 0x4b, 0x09, 0x02, 0x0a, 0x09, 0x00,
        ];
        assert_eq!(bytes, expect);
        assert_eq!(
            sha256(&a.image.code),
            hex32("0fe5a198c3048319789344d79fba856c45e178446346f1417fca0eed7cacf015")
        );
    }

    #[test]
    fn assemble_ok_runs_halt_ok() {
        let a = assemble(OK_IR).unwrap();
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
        assert_eq!(r.cycles, 7);
    }

    #[test]
    fn refuse_unknown_mnemonic() {
        let err = assemble(
            ".caps Compute\n.budget cycles=8 out=0 pages=1\n.entry s\ns:\n    bogon\n",
        )
        .unwrap_err();
        assert!(err.message.contains("unknown mnemonic"), "{err}");
    }

    fn hex32(s: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        for (i, b) in out.iter_mut().enumerate() {
            *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
        }
        out
    }
}

#[cfg(test)]
mod policy_examples {
    use super::*;
    use bilkrnl_core::cap::Cap;
    use bilkrnl_core::digest::hex32 as fmt_hex;
    use bilkrnl_core::enclave::artifact_digest;
    use bilkrnl_core::isa::{ExitReason, Vm};
    use std::fs;
    use std::path::PathBuf;

    fn example(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples").join(name)
    }

    fn run_example(name: &str) -> (BytecodeImage, bilkrnl_core::isa::EnclaveResult) {
        let src = fs::read_to_string(example(name)).unwrap();
        let a = assemble(&src).unwrap();
        let mut vm = Vm::new(
            &a.image.code,
            a.image.entry,
            a.image.budget,
            a.image.initial_caps,
        )
        .unwrap();
        let r = vm.run();
        (a.image, r)
    }

    #[test]
    fn policy_pass_ratifies_shape() {
        let (img, r) = run_example("policy_pass.bil1.ir");
        assert_eq!(img.entry, 1);
        assert_eq!(r.exit, ExitReason::Halt);
        assert_eq!(r.out, b"PASS\n");
        assert_eq!(r.caps_at_exit, Cap::Compute);
        assert_eq!(r.cycles, 15);
        // Printed once when regenerating goldens: cargo test -p bil1asm policy_pass -- --nocapture
        eprintln!(
            "policy_pass code={} out={} art={}",
            fmt_hex(&r.code_digest),
            fmt_hex(&r.out_digest),
            fmt_hex(&artifact_digest(&r))
        );
    }

    #[test]
    fn policy_fail_emits_fail() {
        let (_img, r) = run_example("policy_fail.bil1.ir");
        assert_eq!(r.exit, ExitReason::Halt);
        assert_eq!(r.out, b"FAIL\n");
        assert_eq!(r.caps_at_exit, Cap::Compute);
        assert_eq!(r.cycles, 21);
        eprintln!(
            "policy_fail code={} out={} art={}",
            fmt_hex(&r.code_digest),
            fmt_hex(&r.out_digest),
            fmt_hex(&artifact_digest(&r))
        );
    }

    #[test]
    fn example_ok_ir_matches_golden_image() {
        let src = fs::read_to_string(example("ok.bil1.ir")).unwrap();
        let a = assemble(&src).unwrap();
        let expect: [u8; 42] = [
            0x42, 0x49, 0x4c, 0x31, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x02, 0x00,
            0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x02, 0x4f, 0x09, 0x02, 0x4b, 0x09, 0x02, 0x0a, 0x09, 0x00,
        ];
        assert_eq!(a.image.encode(), expect);
    }

    #[test]
    fn policy_pass_ir_matches_core_fixture() {
        let src = fs::read_to_string(example("policy_pass.bil1.ir")).unwrap();
        let a = assemble(&src).unwrap();
        let (entry, code) = bilkrnl_core::isa::fixture_policy_pass_program();
        assert_eq!(a.image.entry, entry);
        assert_eq!(a.image.code, code);
    }

    #[test]
    fn policy_fail_ir_matches_core_fixture() {
        let src = fs::read_to_string(example("policy_fail.bil1.ir")).unwrap();
        let a = assemble(&src).unwrap();
        let (entry, code) = bilkrnl_core::isa::fixture_policy_fail_program();
        assert_eq!(a.image.entry, entry);
        assert_eq!(a.image.code, code);
    }
}
