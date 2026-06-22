//! clihatch: scaffold a complete, clispec-compliant, agent-facing Rust CLI -
//! source skeleton, schema + conformance test, and the GitHub-hosted
//! dual-publish release pipeline.
//!
//! The whole pipeline is reachable through [`run`], which the CLI and tests
//! both use.

mod error;
mod github;
mod output;
mod process;
mod scaffold;
pub mod schema;
mod secrets;

pub use error::ClihatchError;
pub use output::{next_steps, render, render_secrets, render_verify};
pub use scaffold::{Outcome, Vars, scaffold};
pub use secrets::{
    RealSecretOps, SecretOps, SecretReport, Skip, Sources, VerifyReport, bootstrap,
    cargo_token_from_credentials, pypi_token_from_pypirc, verify,
};

use std::path::PathBuf;

/// Rendered output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

/// A request to scaffold a new CLI crate.
#[derive(Debug, Clone)]
pub struct Request {
    pub name: String,
    pub description: String,
    pub owner: String,
    pub author: String,
    pub year: String,
    /// Directory the new `<name>/` crate is created inside.
    pub into: PathBuf,
    pub git: bool,
    /// Create the GitHub repo (`owner/name`) and push the initial commit.
    pub github: bool,
    /// Include the PyPI/maturin dual-publish machinery.
    pub pypi: bool,
}

/// Scaffold a new crate from the request, optionally creating its GitHub repo.
pub fn run(req: &Request) -> Result<Outcome, ClihatchError> {
    validate_name(&req.name)?;
    if req.github && !req.git {
        return Err(ClihatchError::Usage {
            message: "--github needs the initial commit; remove --no-git".into(),
        });
    }
    let vars = Vars::new(
        &req.name,
        &req.description,
        &req.owner,
        &req.author,
        &req.year,
        req.pypi,
    );
    let mut outcome = scaffold(&req.into, &vars, req.git)?;
    if req.github {
        let dir = req.into.join(&req.name);
        outcome.repo = Some(github::create_repo(
            &process::SystemRunner,
            &req.owner,
            &req.name,
            &req.description,
            &dir,
        )?);
    }
    Ok(outcome)
}

/// The conventional Homebrew tap for a repo's owner: `<owner>/homebrew-tap`.
/// Keeps `clihatch secrets`' default consistent with the tap the generated
/// release workflow clones (`<owner>/homebrew-tap`).
pub fn default_tap(repo: &str) -> String {
    let owner = repo.split('/').next().unwrap_or(repo);
    format!("{owner}/homebrew-tap")
}

/// Crate-name rules: `[a-z][a-z0-9_-]*`, matching what crates.io accepts.
fn validate_name(name: &str) -> Result<(), ClihatchError> {
    let ok = name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(ClihatchError::Usage {
            message: format!(
                "invalid crate name {name:?}: use lowercase letters, digits, '-' or '_', starting with a letter"
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Request, run, validate_name};
    use std::path::PathBuf;

    #[test]
    fn github_without_git_is_a_usage_error() {
        let req = Request {
            name: "demo".into(),
            description: "d".into(),
            owner: "rvben".into(),
            author: "A <a@b.c>".into(),
            year: "2026".into(),
            into: PathBuf::from("/tmp/does-not-matter"),
            git: false,
            github: true,
            pypi: true,
        };
        let err = run(&req).expect_err("--github with --no-git must fail");
        assert_eq!(err.kind(), "usage");
        // Fails fast, before touching the filesystem.
        assert!(!PathBuf::from("/tmp/does-not-matter/demo").exists());
    }

    #[test]
    fn default_tap_follows_repo_owner() {
        assert_eq!(super::default_tap("rvben/dotdiff"), "rvben/homebrew-tap");
        assert_eq!(super::default_tap("acme/tool"), "acme/homebrew-tap");
    }

    #[test]
    fn accepts_good_names() {
        assert!(validate_name("dotpick").is_ok());
        assert!(validate_name("my-cool-tool2").is_ok());
        assert!(validate_name("a_b").is_ok());
    }

    #[test]
    fn rejects_bad_names() {
        assert!(validate_name("").is_err());
        assert!(validate_name("9lives").is_err());
        assert!(validate_name("Caps").is_err());
        assert!(validate_name("has space").is_err());
        assert!(validate_name("dots.bad").is_err());
    }
}
