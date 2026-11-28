//! bil1asm — assemble / dump / lower BIL1 (host-only).

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitCode};

use bil1asm::{
    assemble, disassemble, find_bil_bin, is_bil_source, lower_stack_bc, run_bil_verify,
};
use bilkrnl_core::image::BytecodeImage;

fn usage() -> ! {
    eprintln!(
        "usage:\n  \
         bil1asm assemble <prog.bil1.ir> -o <out.bil1>\n  \
         bil1asm dump <img.bil1>\n  \
         bil1asm lower <prog.bc.txt|.bil> -o <out.bil1> [--skip-verify] [--allow-bounded]\n  \
         bil1asm --help"
    );
    std::process::exit(2);
}

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "assemble" => cmd_assemble(&args),
        "dump" => cmd_dump(&args),
        "lower" => cmd_lower(&args),
        other => {
            eprintln!("error: unknown subcommand '{other}'");
            usage();
        }
    }
}

fn cmd_assemble(args: &[String]) -> ExitCode {
    let mut input: Option<&str> = None;
    let mut output: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: -o needs a path");
                    return ExitCode::from(2);
                }
                output = Some(args[i].as_str());
            }
            s if !s.starts_with('-') => {
                if input.is_some() {
                    eprintln!("error: unexpected argument '{s}'");
                    return ExitCode::from(2);
                }
                input = Some(s);
            }
            s => {
                eprintln!("error: unknown flag '{s}'");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let (Some(input), Some(output)) = (input, output) else {
        eprintln!("error: assemble needs <prog.bil1.ir> -o <out.bil1>");
        return ExitCode::from(2);
    };

    let src = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: reading {input}: {e}");
            return ExitCode::from(1);
        }
    };
    let assembled = match assemble(&src) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes = assembled.image.encode();
    if let Err(e) = write_new(Path::new(output), &bytes) {
        eprintln!("error: writing {output}: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn cmd_dump(args: &[String]) -> ExitCode {
    if args.len() != 1 {
        eprintln!("error: dump needs <img.bil1>");
        return ExitCode::from(2);
    }
    let bytes = match fs::read(&args[0]) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: reading {}: {e}", args[0]);
            return ExitCode::from(1);
        }
    };
    let img = match BytecodeImage::decode(&bytes) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: decode: {e:?}");
            return ExitCode::from(1);
        }
    };
    let _ = io::stdout().write_all(disassemble(&img).as_bytes());
    ExitCode::SUCCESS
}

fn cmd_lower(args: &[String]) -> ExitCode {
    let mut input: Option<&str> = None;
    let mut output: Option<&str> = None;
    let mut skip_verify = false;
    // Default: forward --allow-bounded so bounded proofs do not fail the gate.
    let mut allow_bounded = true;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: -o needs a path");
                    return ExitCode::from(2);
                }
                output = Some(args[i].as_str());
            }
            "--skip-verify" => skip_verify = true,
            "--allow-bounded" => allow_bounded = true,
            s if !s.starts_with('-') => {
                if input.is_some() {
                    eprintln!("error: unexpected argument '{s}'");
                    return ExitCode::from(2);
                }
                input = Some(s);
            }
            s => {
                eprintln!("error: unknown flag '{s}'");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let (Some(input), Some(output)) = (input, output) else {
        eprintln!(
            "error: lower needs <prog.bc.txt|.bil> -o <out.bil1> [--skip-verify] [--allow-bounded]"
        );
        return ExitCode::from(2);
    };

    let input_path = Path::new(input);
    // B3.3: `.bil` sources must pass `bil verify` before dump-bytecode / lower.
    if is_bil_source(input_path) && !skip_verify {
        if let Err(e) = run_bil_verify(input_path, allow_bounded) {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    }

    let bc_text = match load_stack_bc(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let assembled = match lower_stack_bc(&bc_text) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes = assembled.image.encode();
    if let Err(e) = write_new(Path::new(output), &bytes) {
        eprintln!("error: writing {output}: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// Load `.bc.txt` directly, or shell out to `bil vm --dump-bytecode` for `.bil`.
fn load_stack_bc(path: &Path) -> Result<String, String> {
    if is_bil_source(path) {
        let bil = find_bil_bin().map_err(|e| format!("bil: {e}"))?;
        let out = Command::new(&bil)
            .arg("vm")
            .arg(path)
            .arg("--dump-bytecode")
            .output()
            .map_err(|e| format!("bil: spawning bil ({}): {e}", bil.display()))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            return Err(format!(
                "bil: bil vm --dump-bytecode failed ({}):\n{stderr}{stdout}",
                out.status
            ));
        }
        // bil prints dump to stdout (and may also print `name = value` result lines).
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    // Refuse silent overwrite of a different file; allow identical rewrite.
    if path.exists() {
        let existing = fs::read(path)?;
        if existing == bytes {
            return Ok(());
        }
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "refusing to overwrite existing file with different contents (delete it first)",
        ));
    }
    fs::write(path, bytes)
}
