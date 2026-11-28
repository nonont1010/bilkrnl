//! Parse `bil vm --dump-bytecode` Debug text (and a checked-in `.bc.txt` dialect).

use std::fmt;

/// Parsed stack-bytecode program (one or more functions + optional header).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackProgram {
    pub caps: Option<String>,
    pub budget_cycles: Option<u64>,
    pub budget_out: Option<u32>,
    pub budget_pages: Option<u16>,
    /// Preferred entry function name from `.fn name` directive (without `():`).
    pub entry_fn: Option<String>,
    pub functions: Vec<StackFn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackFn {
    pub name: String,
    pub params: Vec<String>,
    pub ops: Vec<StackOp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StackOp {
    PushInt(i64),
    PushFloat(String),
    PushBool(bool),
    PushStr(String),
    LoadVar(String),
    StoreVar(String),
    PopVar(String),
    Unary(String),
    Binary(String),
    MakeList(usize),
    MakeStruct {
        concept: String,
        fields: Vec<String>,
    },
    UpdateStruct {
        fields: Vec<String>,
    },
    GetField(String),
    Call {
        func: String,
        argc: usize,
    },
    PipeStage(String),
    /// Nested / unsupported shapes retained for diagnostics.
    GuardedSelect,
    GuardedLoop,
    Match,
    Quantifier,
    Return,
    /// Raw Debug text we could not classify further.
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse bil dump-bytecode text or a `.bc.txt` dialect into a [`StackProgram`].
pub fn parse_stack_bc(source: &str) -> Result<StackProgram, ParseError> {
    let mut prog = StackProgram {
        caps: None,
        budget_cycles: None,
        budget_out: None,
        budget_pages: None,
        entry_fn: None,
        functions: Vec::new(),
    };

    let mut current: Option<StackFn> = None;

    for (idx, raw) in source.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = strip_comment(raw).trim();
        if trimmed.is_empty() {
            continue;
        }
        // Banner from bil dump.
        if trimmed.starts_with("===") {
            continue;
        }

        // Optional header directives (wrapper dialect).
        if let Some(rest) = trimmed.strip_prefix('.') {
            let (dir, args) = split_first(rest.trim_start_matches(char::is_whitespace));
            // `.fn name():` or `.fn name:` or indented `  .fn name():`
            if dir == "fn" || rest.trim_start().starts_with("fn ") {
                let fn_line = if dir == "fn" {
                    args
                } else {
                    rest.trim_start()
                        .strip_prefix("fn")
                        .unwrap_or(rest)
                        .trim()
                };
                flush_fn(&mut prog, &mut current);
                let (name, params) = parse_fn_header(fn_line, line_no)?;
                // Bare `.fn name` (no colon / no ops yet) sets preferred entry.
                if !fn_line.contains(':') && params.is_empty() && !fn_line.contains('(') {
                    prog.entry_fn = Some(name.clone());
                    // Still open a function body for subsequent op lines.
                }
                current = Some(StackFn {
                    name,
                    params,
                    ops: Vec::new(),
                });
                continue;
            }
            match dir {
                "caps" => {
                    let c = args.trim();
                    if c.is_empty() {
                        return Err(ParseError {
                            line: line_no,
                            message: "expected `.caps <Cap>`".into(),
                        });
                    }
                    prog.caps = Some(c.to_string());
                }
                "budget" => parse_budget_directive(args, &mut prog, line_no)?,
                "global" => {
                    // Ignore globals for profile; reject later if ops appear.
                    flush_fn(&mut prog, &mut current);
                    current = Some(StackFn {
                        name: format!("__global_{}", args.trim().trim_end_matches(':')),
                        params: Vec::new(),
                        ops: Vec::new(),
                    });
                }
                "entry" => {
                    // Alias for selecting the entry function by name.
                    let name = args.trim().trim_end_matches(':');
                    if name.is_empty() {
                        return Err(ParseError {
                            line: line_no,
                            message: "expected `.entry <fn>`".into(),
                        });
                    }
                    prog.entry_fn = Some(name.to_string());
                }
                other => {
                    return Err(ParseError {
                        line: line_no,
                        message: format!("unknown directive '.{other}'"),
                    });
                }
            }
            continue;
        }

        // Indented `.fn` from bil dump: `  .fn emit():`
        if let Some(rest) = trimmed.strip_prefix(".fn ") {
            flush_fn(&mut prog, &mut current);
            let (name, params) = parse_fn_header(rest, line_no)?;
            current = Some(StackFn {
                name,
                params,
                ops: Vec::new(),
            });
            continue;
        }

        // Op line: `0000: PushInt(79)` or bare `PushInt(79)`.
        let op_text = if let Some((_, after)) = split_addr_prefix(trimmed) {
            after
        } else {
            trimmed
        };

        let op = parse_op(op_text, line_no)?;
        match current.as_mut() {
            Some(f) => f.ops.push(op),
            None => {
                // Implicit function for bare op lists.
                current = Some(StackFn {
                    name: "main".into(),
                    params: Vec::new(),
                    ops: vec![op],
                });
            }
        }
    }

    flush_fn(&mut prog, &mut current);

    if prog.functions.is_empty() {
        return Err(ParseError {
            line: 0,
            message: "no stack functions found".into(),
        });
    }

    Ok(prog)
}

fn flush_fn(prog: &mut StackProgram, current: &mut Option<StackFn>) {
    if let Some(f) = current.take() {
        prog.functions.push(f);
    }
}

fn parse_fn_header(s: &str, line: usize) -> Result<(String, Vec<String>), ParseError> {
    let s = s.trim().trim_end_matches(':').trim();
    if s.is_empty() {
        return Err(ParseError {
            line,
            message: "expected `.fn <name>`".into(),
        });
    }
    if let Some(paren) = s.find('(') {
        let name = s[..paren].trim();
        if name.is_empty() || !is_ident(name) {
            return Err(ParseError {
                line,
                message: format!("invalid function name in '{s}'"),
            });
        }
        let rest = s[paren + 1..].trim();
        let params_src = rest.strip_suffix(')').unwrap_or(rest);
        let params: Vec<String> = params_src
            .split(',')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        Ok((name.to_string(), params))
    } else if is_ident(s) {
        Ok((s.to_string(), Vec::new()))
    } else {
        Err(ParseError {
            line,
            message: format!("invalid function header '{s}'"),
        })
    }
}

fn parse_budget_directive(
    args: &str,
    prog: &mut StackProgram,
    line: usize,
) -> Result<(), ParseError> {
    for tok in args.split_whitespace() {
        let Some((k, v)) = tok.split_once('=') else {
            return Err(ParseError {
                line,
                message: format!("expected key=value in .budget, got '{tok}'"),
            });
        };
        match k {
            "cycles" => {
                prog.budget_cycles = Some(parse_u64(v, line)?);
            }
            "out" => {
                let n = parse_u64(v, line)?;
                if n > u32::MAX as u64 {
                    return Err(ParseError {
                        line,
                        message: format!("out too large: {n}"),
                    });
                }
                prog.budget_out = Some(n as u32);
            }
            "pages" => {
                let n = parse_u64(v, line)?;
                if n == 0 || n > u16::MAX as u64 {
                    return Err(ParseError {
                        line,
                        message: "pages must be 1..=65535".into(),
                    });
                }
                prog.budget_pages = Some(n as u16);
            }
            other => {
                return Err(ParseError {
                    line,
                    message: format!("unknown budget key '{other}'"),
                });
            }
        }
    }
    Ok(())
}

fn parse_op(text: &str, line: usize) -> Result<StackOp, ParseError> {
    let text = text.trim();
    if text == "Return" {
        return Ok(StackOp::Return);
    }
    if text.starts_with("GuardedSelect") {
        return Ok(StackOp::GuardedSelect);
    }
    if text.starts_with("GuardedLoop") {
        return Ok(StackOp::GuardedLoop);
    }
    if text.starts_with("Match") {
        return Ok(StackOp::Match);
    }
    if text.starts_with("Quantifier") {
        return Ok(StackOp::Quantifier);
    }
    if text.starts_with("PipeStage") {
        return Ok(StackOp::PipeStage(text.to_string()));
    }
    if text.starts_with("TrapAbort") {
        return Ok(StackOp::Unknown("TrapAbort".into()));
    }

    if let Some(inner) = unwrap_paren(text, "PushInt") {
        let v = parse_i64(inner, line)?;
        return Ok(StackOp::PushInt(v));
    }
    if let Some(inner) = unwrap_paren(text, "PushFloat") {
        return Ok(StackOp::PushFloat(inner.trim().to_string()));
    }
    if let Some(inner) = unwrap_paren(text, "PushBool") {
        let v = match inner.trim() {
            "true" => true,
            "false" => false,
            other => {
                return Err(ParseError {
                    line,
                    message: format!("invalid PushBool '{other}'"),
                });
            }
        };
        return Ok(StackOp::PushBool(v));
    }
    if let Some(inner) = unwrap_paren(text, "PushStr") {
        return Ok(StackOp::PushStr(unquote(inner, line)?));
    }
    if let Some(inner) = unwrap_paren(text, "LoadVar") {
        return Ok(StackOp::LoadVar(unquote(inner, line)?));
    }
    if let Some(inner) = unwrap_paren(text, "StoreVar") {
        return Ok(StackOp::StoreVar(unquote(inner, line)?));
    }
    if let Some(inner) = unwrap_paren(text, "PopVar") {
        return Ok(StackOp::PopVar(unquote(inner, line)?));
    }
    if let Some(inner) = unwrap_paren(text, "Unary") {
        return Ok(StackOp::Unary(inner.trim().to_string()));
    }
    if let Some(inner) = unwrap_paren(text, "Binary") {
        return Ok(StackOp::Binary(inner.trim().to_string()));
    }
    if let Some(inner) = unwrap_paren(text, "MakeList") {
        let n = parse_u64(inner.trim(), line)? as usize;
        return Ok(StackOp::MakeList(n));
    }
    if let Some(inner) = unwrap_paren(text, "GetField") {
        return Ok(StackOp::GetField(unquote(inner, line)?));
    }

    // Call { func: "bil1_out", argc: 1 }
    if let Some(rest) = text.strip_prefix("Call") {
        let rest = rest.trim();
        if rest.starts_with('{') {
            let func = extract_field(rest, "func", line)?;
            let argc_s = extract_field(rest, "argc", line)?;
            let argc = parse_u64(&argc_s, line)? as usize;
            return Ok(StackOp::Call {
                func: unquote(&func, line).unwrap_or(func),
                argc,
            });
        }
    }

    // MakeStruct { concept: "Foo", fields: ["a", "b"] } — best-effort
    if text.starts_with("MakeStruct") {
        return Ok(StackOp::MakeStruct {
            concept: String::new(),
            fields: Vec::new(),
        });
    }
    if text.starts_with("UpdateStruct") {
        return Ok(StackOp::UpdateStruct {
            fields: Vec::new(),
        });
    }

    Ok(StackOp::Unknown(text.to_string()))
}

fn unwrap_paren<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(name)?.trim_start();
    let rest = rest.strip_prefix('(')?;
    let rest = rest.strip_suffix(')')?;
    Some(rest)
}

fn extract_field(brace_body: &str, field: &str, line: usize) -> Result<String, ParseError> {
    // Accept `{ func: "x", argc: 1 }` style.
    let key = format!("{field}:");
    let Some(pos) = brace_body.find(&key) else {
        return Err(ParseError {
            line,
            message: format!("missing field '{field}' in '{brace_body}'"),
        });
    };
    let after = brace_body[pos + key.len()..].trim_start();
    // Value runs until comma or closing brace.
    let end = after
        .find([',', '}'])
        .unwrap_or(after.len());
    Ok(after[..end].trim().to_string())
}

fn unquote(s: &str, line: usize) -> Result<String, ParseError> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        return Ok(inner.to_string());
    }
    // Already bare ident.
    if is_ident(s) {
        return Ok(s.to_string());
    }
    Err(ParseError {
        line,
        message: format!("expected quoted string, got '{s}'"),
    })
}

fn split_addr_prefix(line: &str) -> Option<(u32, &str)> {
    // `0000: Op...` or `  0000: Op...`
    let line = line.trim();
    let colon = line.find(':')?;
    let addr = line[..colon].trim();
    if addr.is_empty() || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    // Avoid treating `Call { func: ...` as addr — func isn't all hex, but "Call" isn't either.
    // Hex-only address tokens are typically 4 digits; reject if the left side has spaces.
    if addr.contains(char::is_whitespace) {
        return None;
    }
    let n = u32::from_str_radix(addr, 16).ok()?;
    Some((n, line[colon + 1..].trim()))
}

fn parse_i64(s: &str, line: usize) -> Result<i64, ParseError> {
    let s = s.trim();
    i64::from_str_radix(s, 10).map_err(|_| ParseError {
        line,
        message: format!("invalid integer '{s}'"),
    })
}

fn parse_u64(s: &str, line: usize) -> Result<u64, ParseError> {
    let s = s.trim();
    let (radix, digits) = if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (16, h)
    } else {
        (10, s)
    };
    u64::from_str_radix(digits, radix).map_err(|_| ParseError {
        line,
        message: format!("invalid integer '{s}'"),
    })
}

fn strip_comment(line: &str) -> &str {
    // bil Debug dump has no `;` comments in ops; wrapper dialect allows them.
    // Do not strip `;` inside strings — profile dump has none in strings that matter.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bil_dump_emit_ok() {
        let src = r#"
=== BIL Bytecode VM Image (1 functions, 0 globals, 10 instructions, 0 ghost items erased) ===
  .fn emit_ok():
    0000: PushInt(79)
    0001: StoreVar("a")
    0002: PushInt(75)
    0003: StoreVar("b")
    0004: LoadVar("a")
    0005: LoadVar("b")
    0006: Binary(Add)
    0007: PopVar("b")
    0008: PopVar("a")
    0009: Return
"#;
        let p = parse_stack_bc(src).unwrap();
        assert_eq!(p.functions.len(), 1);
        assert_eq!(p.functions[0].name, "emit_ok");
        assert_eq!(p.functions[0].ops.len(), 10);
        assert_eq!(p.functions[0].ops[0], StackOp::PushInt(79));
        assert_eq!(p.functions[0].ops[6], StackOp::Binary("Add".into()));
        assert_eq!(p.functions[0].ops[9], StackOp::Return);
    }

    #[test]
    fn parse_call_and_directives() {
        let src = r#"
.caps IoOutBounded
.budget cycles=64 out=8 pages=1
.fn emit
0000: PushInt(79)
0001: Call { func: "bil1_out", argc: 1 }
0002: Return
"#;
        let p = parse_stack_bc(src).unwrap();
        assert_eq!(p.caps.as_deref(), Some("IoOutBounded"));
        assert_eq!(p.budget_cycles, Some(64));
        assert_eq!(p.entry_fn.as_deref(), Some("emit"));
        assert_eq!(
            p.functions[0].ops[1],
            StackOp::Call {
                func: "bil1_out".into(),
                argc: 1
            }
        );
    }
}
