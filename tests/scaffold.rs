//! Offline scaffolding tests: exercise `run()` into a temp dir and assert the
//! tree, full placeholder substitution, and the refuse-if-exists contract.

use std::fs;
use std::path::{Path, PathBuf};

use clihatch::{ClihatchError, Request, run};

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clihatch-test-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn request(into: &Path, name: &str) -> Request {
    Request {
        name: name.to_string(),
        description: "A test tool.".into(),
        owner: "rvben".into(),
        author: "Tester <t@example.com>".into(),
        year: "2026".into(),
        into: into.to_path_buf(),
        git: false,
        github: false,
        pypi: true,
    }
}

fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_files(&p, out);
        } else {
            out.push(p);
        }
    }
}

#[test]
fn scaffolds_the_expected_tree_with_no_leftover_placeholders() {
    let base = temp_dir().join("tree");
    let _ = fs::create_dir_all(&base);
    let outcome = run(&request(&base, "neat-tool")).expect("scaffold");
    let crate_dir = base.join("neat-tool");

    for expected in [
        "Cargo.toml",
        "pyproject.toml",
        "Makefile",
        "README.md",
        "LICENSE",
        ".gitignore",
        "prek.toml",
        "src/main.rs",
        "src/lib.rs",
        "src/error.rs",
        "src/schema.rs",
        "schemas/clispec-v0.2.json",
        "tests/conformance.rs",
        "tests/cli.rs",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
    ] {
        assert!(
            crate_dir.join(expected).exists(),
            "missing generated file: {expected}"
        );
    }

    // Substitution sanity: the package name made it in...
    let cargo = fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("name = \"neat-tool\""));
    // ...the snake-case crate is used in main.rs...
    let main = fs::read_to_string(crate_dir.join("src/main.rs")).unwrap();
    assert!(main.contains("use neat_tool::"));

    // ...and none of clihatch's own placeholders survive anywhere. (GitHub
    // Actions `${{ ... }}` expressions are left intact and must not trip this.)
    let placeholders = [
        "{{name}}",
        "{{name_snake}}",
        "{{name_pascal}}",
        "{{description}}",
        "{{owner}}",
        "{{author}}",
        "{{year}}",
        // Conditional-block markers must never survive rendering.
        "{{#",
        "{{/",
        "{{^",
    ];
    let mut files = Vec::new();
    walk_files(&crate_dir, &mut files);
    for f in &files {
        let content = fs::read_to_string(f).unwrap_or_default();
        for ph in placeholders {
            assert!(
                !content.contains(ph),
                "unsubstituted {ph} in {}",
                f.display()
            );
        }
    }

    assert!(outcome.files.len() >= 16);
    assert!(!outcome.committed);
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn no_pypi_omits_pyproject_and_pypi_workflow_jobs() {
    let base = temp_dir().join("nopypi");
    let _ = fs::create_dir_all(&base);
    let mut req = request(&base, "rusttool");
    req.pypi = false;
    let outcome = run(&req).expect("scaffold");
    let crate_dir = base.join("rusttool");

    // pyproject.toml is omitted entirely.
    assert!(
        !crate_dir.join("pyproject.toml").exists(),
        "pyproject.toml must be omitted with --no-pypi"
    );
    assert!(!outcome.files.iter().any(|f| f == "pyproject.toml"));

    // The release workflow keeps crates.io + Homebrew but drops all PyPI/maturin
    // machinery, and leaves no unrendered conditional markers.
    let release = fs::read_to_string(crate_dir.join(".github/workflows/release.yml")).unwrap();
    let lower = release.to_lowercase();
    for needle in ["pypi", "maturin", "wheel", "sdist"] {
        assert!(
            !lower.contains(needle),
            "release.yml should not mention {needle:?} with --no-pypi"
        );
    }
    assert!(!release.contains("{{#") && !release.contains("{{/"));
    assert!(release.contains("publish-crates"));
    assert!(release.contains("update-homebrew"));
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn git_scaffold_lands_on_main_branch() {
    // The generated CI triggers on `main`, so the initial commit must be there
    // regardless of the host's init.defaultBranch.
    let base = temp_dir().join("branch");
    let _ = fs::create_dir_all(&base);
    let mut req = request(&base, "branchtool");
    req.git = true;
    let outcome = run(&req).expect("scaffold");
    assert!(outcome.committed);
    let crate_dir = base.join("branchtool");
    let branch = std::process::Command::new("git")
        .current_dir(&crate_dir)
        .args(["branch", "--show-current"])
        .output()
        .expect("git");
    assert_eq!(String::from_utf8_lossy(&branch.stdout).trim(), "main");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn refuses_to_overwrite_an_existing_directory() {
    let base = temp_dir().join("exists");
    let _ = fs::create_dir_all(base.join("taken"));
    let err = run(&request(&base, "taken")).unwrap_err();
    assert!(matches!(err, ClihatchError::Exists { .. }));
    assert_eq!(err.exit_code(), 3);
    let _ = fs::remove_dir_all(&base);
}
