//! The deep guard: scaffold a crate and `cargo check` it, proving the
//! templates produce code that actually compiles. Tolerant of an offline /
//! unavailable registry (skips), but a genuine compile error fails loudly.

use std::fs;
use std::process::Command;

use clihatch::{Request, run};

#[test]
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
        .args(["check", "--quiet"])
        .output()
        .expect("run cargo");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = fs::remove_dir_all(&base);

    if out.status.success() {
        return;
    }
    let networkish = [
        "failed to download",
        "failed to get",
        "Unable to update registry",
        "Blocking waiting",
        "could not fetch",
        "no matching package",
        "failed to load source",
        "spurious network error",
    ];
    if networkish.iter().any(|m| stderr.contains(m)) {
        eprintln!("skipping (registry unavailable):\n{stderr}");
        return;
    }
    panic!("generated crate failed to compile:\n{stderr}");
}
