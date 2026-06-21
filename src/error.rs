//! Error type, the stable error `kind` set, and the exit-code contract.
//!
//! Errors are reported as a clispec structured envelope on the last line of
//! stderr.
//!
//! Exit codes (also declared in the schema):
//! - `2` an IO or git failure
//! - `3` usage error (bad arguments, or the target directory already exists)

use thiserror::Error;

/// All failure modes of a clihatch run.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClihatchError {
    /// Invalid command-line arguments (also wraps clap errors).
    #[error("{message}")]
    Usage { message: String },

    /// The target directory already exists (clihatch never overwrites).
    #[error("{path} already exists")]
    Exists { path: String },

    /// A filesystem operation failed.
    #[error("{message}")]
    Io { message: String },

    /// A git operation failed.
    #[error("git: {message}")]
    Git { message: String },

    /// An external tool (gh, ssh-keygen) failed.
    #[error("{tool}: {message}")]
    Backend { tool: &'static str, message: String },
}

impl ClihatchError {
    /// Stable snake_case identifier consumers branch on (the schema `errors` set).
    pub fn kind(&self) -> &'static str {
        match self {
            ClihatchError::Usage { .. } => "usage",
            ClihatchError::Exists { .. } => "exists",
            ClihatchError::Io { .. } => "io",
            ClihatchError::Git { .. } => "git",
            ClihatchError::Backend { .. } => "backend",
        }
    }

    /// Actionable remediation, when there is one.
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            ClihatchError::Usage { .. } => Some("see `clihatch --help` or `clihatch schema`"),
            ClihatchError::Exists { .. } => {
                Some("choose another name, or remove the existing directory")
            }
            ClihatchError::Backend { .. } => {
                Some("ensure `gh` is installed and authenticated (`gh auth status`)")
            }
            _ => None,
        }
    }

    /// The process exit code associated with this error.
    pub fn exit_code(&self) -> i32 {
        match self {
            ClihatchError::Io { .. }
            | ClihatchError::Git { .. }
            | ClihatchError::Backend { .. } => 2,
            ClihatchError::Usage { .. } | ClihatchError::Exists { .. } => 3,
        }
    }
}
