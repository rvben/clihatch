//! The deep guard: scaffold a crate and `cargo check` it, proving the
//! templates produce code that actually compiles with the shipped lockfile.
//! Run explicitly in Cargo CI; Nix tests generated projects hermetically through
//! checks.generated instead of downloading dependencies inside its sandbox.

use std::fs;
use std::process::Command;

use clihatch::{Request, run};

#[test]
#[ignore = "requires a populated Cargo cache or registry access; run explicitly in CI"]
fn generated_crate_compiles() {
    let base = std::env::temp_dir().join(format!("clihatch-compile-{}", std::process::id()));
    let _ = fs::create_dir_all(&base);
    let req = Request {
        name: "buildcheck".into(),
        description: "build check".into(),
        owner: "rvben".into(),
        author: "A <a@b.c>".into(),
        year: "2026".into(),
        into: base.clone(),
        git: false,
        github: false,
        pypi: true,
    };
    run(&req).expect("scaffold");
    let crate_dir = base.join("buildcheck");

    let out = Command::new("cargo")
        .current_dir(&crate_dir)
        // Do not contend with the outer Cargo invocation's target-directory lock.
        .env("CARGO_TARGET_DIR", crate_dir.join("target"))
        .args(["check", "--locked", "--quiet"])
        .output()
        .expect("run cargo");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = fs::remove_dir_all(&base);

    if out.status.success() {
        return;
    }
    panic!("generated crate failed to compile:\n{stderr}");
}
