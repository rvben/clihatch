//! clihatch: scaffold a complete, clispec-compliant, agent-facing Rust CLI -
//! source skeleton, schema + conformance test, and the GitHub-hosted
//! dual-publish release pipeline.
//!
//! The whole pipeline is reachable through [`run`], which the CLI and tests
//! both use.

mod error;
mod output;
mod scaffold;
pub mod schema;
mod secrets;

pub use error::ClihatchError;
pub use output::{render, render_secrets};
pub use scaffold::{Outcome, Vars, scaffold};
pub use secrets::{
    RealSecretOps, SecretOps, SecretReport, Skip, Sources, bootstrap, cargo_token_from_credentials,
    pypi_token_from_pypirc,
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
}

/// Scaffold a new crate from the request.
pub fn run(req: &Request) -> Result<Outcome, ClihatchError> {
    validate_name(&req.name)?;
    let vars = Vars::new(
        &req.name,
        &req.description,
        &req.owner,
        &req.author,
        &req.year,
    );
    scaffold(&req.into, &vars, req.git)
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
    use super::validate_name;

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
