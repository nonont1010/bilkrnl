//! B3.3 — run `bil verify` on `.bil` sources before lowering.
//!
//! Refuse to lower when verify fails or `bil` is missing (unless the caller
//! skips the gate). `.bc.txt` inputs have no source and are not gated.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Locate the `bil` host binary: `BIL_BIN`, then `PATH`, then sibling `lang/bil`.
pub fn find_bil_bin() -> Result<PathBuf, String> {
    if let Ok(p) = env::var("BIL_BIN") {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Ok(pb);
        }
        return Err(format!("BIL_BIN={p} is not a file"));
    }
    if let Ok(out) = Command::new("which").arg("bil").output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return Ok(PathBuf::from(p));
            }
        }
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../../lang/bil/target/release/bil"),
        manifest_dir.join("../../../lang/bil/target/release/bil"),
        manifest_dir.join("../../../../lang/bil/target/release/bil"),
    ];
    for c in &candidates {
        if c.is_file() {
            return Ok(c.clone());
        }
    }
    Err(
        "bil binary not found (set BIL_BIN, install bil on PATH, or use a .bc.txt input / --skip-verify)"
            .into(),
    )
}

/// Run `bil verify <source> [--allow-bounded]`.
///
/// On success returns `Ok(())`. On failure returns an error string prefixed
/// with `bil:` (missing binary / spawn) or `verify:` (nonzero exit).
pub fn run_bil_verify(source: &Path, allow_bounded: bool) -> Result<(), String> {
    let bil = find_bil_bin().map_err(|e| format!("bil: {e}"))?;
    let mut cmd = Command::new(&bil);
    cmd.arg("verify").arg(source);
    if allow_bounded {
        cmd.arg("--allow-bounded");
    }
    let out = cmd
        .output()
        .map_err(|e| format!("bil: spawning bil ({}): {e}", bil.display()))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    Err(format!(
        "verify: bil verify failed ({}):\n{stderr}{stdout}",
        out.status
    ))
}

/// True when `path` looks like bil source (`.bil`), so the verify gate applies.
pub fn is_bil_source(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("bil"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    // Serialize env mutations across tests in this module.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn examples_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/lower")
    }

    fn bil_available() -> bool {
        find_bil_bin().is_ok()
    }

    #[test]
    fn find_bil_bin_respects_bil_bin_env() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev = env::var_os("BIL_BIN");
        // Point at a path that is not a file → clear error.
        env::set_var("BIL_BIN", "/nonexistent/bil-binary-for-test");
        let err = find_bil_bin().unwrap_err();
        assert!(err.contains("BIL_BIN="), "{err}");
        match prev {
            Some(v) => env::set_var("BIL_BIN", v),
            None => env::remove_var("BIL_BIN"),
        }
    }

    #[test]
    fn verify_pass_out_hello() {
        let _g = ENV_LOCK.lock().unwrap();
        if !bil_available() {
            eprintln!("skip: bil not found");
            return;
        }
        let src = examples_dir().join("out_hello.bil");
        run_bil_verify(&src, true).expect("out_hello.bil should verify");
    }

    #[test]
    fn verify_fail_refuted_contract() {
        let _g = ENV_LOCK.lock().unwrap();
        if !bil_available() {
            eprintln!("skip: bil not found");
            return;
        }
        let src = examples_dir().join("verify_fail.bil");
        let err = run_bil_verify(&src, true).expect_err("verify_fail.bil must fail");
        assert!(err.starts_with("verify:"), "{err}");
        assert!(
            err.contains("REFUTED") || err.contains("failed"),
            "{err}"
        );
    }

    #[test]
    fn bil_missing_prefixed_error() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev = env::var_os("BIL_BIN");
        env::set_var("BIL_BIN", "/nonexistent/bil-binary-for-test");
        let src = examples_dir().join("out_hello.bil");
        let err = run_bil_verify(&src, true).unwrap_err();
        assert!(err.starts_with("bil:"), "{err}");
        match prev {
            Some(v) => env::set_var("BIL_BIN", v),
            None => env::remove_var("BIL_BIN"),
        }
    }

    #[test]
    fn is_bil_source_extension() {
        assert!(is_bil_source(Path::new("foo.bil")));
        assert!(is_bil_source(Path::new("foo.BIL")));
        assert!(!is_bil_source(Path::new("foo.bc.txt")));
        assert!(!is_bil_source(Path::new("foo.bil1")));
    }

    #[test]
    fn verify_fail_fixture_exists() {
        let src = examples_dir().join("verify_fail.bil");
        assert!(src.is_file(), "missing fixture {}", src.display());
        let body = fs::read_to_string(&src).unwrap();
        assert!(body.contains("ensures"), "fixture should have a contract");
    }
}
