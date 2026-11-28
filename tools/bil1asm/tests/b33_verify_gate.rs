//! B3.3 CLI integration: verify gate on `bil1asm lower` for `.bil` inputs.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn bil1asm() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bil1asm"))
}

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/lower")
}

fn bil_available() -> bool {
    bil1asm::find_bil_bin().is_ok()
}

fn tmp_out(name: &str) -> PathBuf {
    let mut p = env::temp_dir();
    p.push(format!("bil1asm-b33-{name}-{}", std::process::id()));
    let _ = fs::remove_file(&p);
    p
}

#[test]
fn bc_txt_skips_verify_gate() {
    let _g = ENV_LOCK.lock().unwrap();
    let out = tmp_out("bc");
    let status = bil1asm()
        .arg("lower")
        .arg(examples().join("out_hello.bc.txt"))
        .arg("-o")
        .arg(&out)
        .status()
        .expect("spawn bil1asm");
    assert!(status.success(), "lower .bc.txt should succeed without bil verify");
    assert!(out.is_file(), "expected output {}", out.display());
    let _ = fs::remove_file(&out);
}

#[test]
fn bil_verify_fail_writes_nothing() {
    let _g = ENV_LOCK.lock().unwrap();
    if !bil_available() {
        eprintln!("skip: bil not found");
        return;
    }
    let out = tmp_out("fail");
    let result = bil1asm()
        .arg("lower")
        .arg(examples().join("verify_fail.bil"))
        .arg("-o")
        .arg(&out)
        .output()
        .expect("spawn bil1asm");
    assert!(!result.status.success(), "verify_fail.bil must exit nonzero");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("verify:"),
        "expected verify: diagnostic, got:\n{stderr}"
    );
    assert!(
        !out.exists(),
        "must not write {} when verify fails",
        out.display()
    );
}

#[test]
fn bil_verify_pass_lowers() {
    let _g = ENV_LOCK.lock().unwrap();
    if !bil_available() {
        eprintln!("skip: bil not found");
        return;
    }
    let out = tmp_out("pass");
    let status = bil1asm()
        .arg("lower")
        .arg(examples().join("out_hello.bil"))
        .arg("-o")
        .arg(&out)
        .status()
        .expect("spawn bil1asm");
    assert!(status.success(), "out_hello.bil should verify + lower");
    assert!(out.is_file());
    let bytes = fs::read(&out).unwrap();
    assert!(bytes.len() > 32);
    assert_eq!(&bytes[0..4], b"BIL1");
    // Live dump includes StoreVar slots; code still starts with ldi 'O'; out.
    assert_eq!(&bytes[32..35], &[0x02, 0x4f, 0x09]);
    let _ = fs::remove_file(&out);
}

#[test]
fn bil_missing_errors_without_skip_verify() {
    let _g = ENV_LOCK.lock().unwrap();
    let prev = env::var_os("BIL_BIN");
    env::set_var("BIL_BIN", "/nonexistent/bil-binary-b33-test");
    let out = tmp_out("missing");
    let result = bil1asm()
        .arg("lower")
        .arg(examples().join("out_hello.bil"))
        .arg("-o")
        .arg(&out)
        .output()
        .expect("spawn");
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("bil:"), "expected bil: diagnostic:\n{stderr}");
    assert!(!out.exists());
    match prev {
        Some(v) => env::set_var("BIL_BIN", v),
        None => env::remove_var("BIL_BIN"),
    }
}

#[test]
fn skip_verify_still_needs_bil_for_dump() {
    let _g = ENV_LOCK.lock().unwrap();
    let prev = env::var_os("BIL_BIN");
    env::set_var("BIL_BIN", "/nonexistent/bil-binary-b33-test");
    let out = tmp_out("skip");
    let result = bil1asm()
        .arg("lower")
        .arg(examples().join("out_hello.bil"))
        .arg("-o")
        .arg(&out)
        .arg("--skip-verify")
        .output()
        .expect("spawn");
    // Gate skipped, but dump-bytecode still needs bil → fail; no output.
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("bil:"),
        "dump path should still need bil:\n{stderr}"
    );
    assert!(!out.exists());
    match prev {
        Some(v) => env::set_var("BIL_BIN", v),
        None => env::remove_var("BIL_BIN"),
    }
}

#[test]
fn skip_verify_with_bil_lowers() {
    let _g = ENV_LOCK.lock().unwrap();
    if !bil_available() {
        eprintln!("skip: bil not found");
        return;
    }
    let out = tmp_out("skip-ok");
    // Escape hatch: skip gate but still dump+lower via bil.
    let status = bil1asm()
        .arg("lower")
        .arg(examples().join("out_hello.bil"))
        .arg("-o")
        .arg(&out)
        .arg("--skip-verify")
        .status()
        .expect("spawn");
    assert!(status.success());
    assert!(Path::new(&out).is_file());
    let _ = fs::remove_file(&out);
}
