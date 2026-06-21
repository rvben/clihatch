//! Bootstrap a repo's three release secrets:
//! - `HOMEBREW_TAP_DEPLOY_KEY` - generate an ed25519 key, register it as a
//!   write deploy key on the tap, store the private key (fully automated);
//! - `CARGO_REGISTRY_TOKEN` - from `~/.cargo/credentials.toml`;
//! - `PYPI_API_TOKEN` - from the environment / stdin.
//!
//! [`SecretOps`] is the seam: the real backend shells out to `gh` and
//! `ssh-keygen`; tests substitute a fake and exercise the orchestration
//! offline.

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use serde::Serialize;

use crate::error::ClihatchError;

/// External side effects a secrets run performs.
pub trait SecretOps {
    fn set_secret(&self, repo: &str, name: &str, value: &str) -> Result<(), ClihatchError>;
    /// Generate an ed25519 keypair, returning `(private_key, public_key)`.
    fn generate_keypair(&self, comment: &str) -> Result<(String, String), ClihatchError>;
    fn add_deploy_key(&self, tap: &str, title: &str, public_key: &str)
    -> Result<(), ClihatchError>;
}

/// The tokens and targets a run draws on.
#[derive(Debug, Clone)]
pub struct Sources {
    pub crate_name: String,
    pub tap: String,
    pub cargo_token: Option<String>,
    pub pypi_token: Option<String>,
}

/// A secret that was not set, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skip {
    pub secret: String,
    pub reason: String,
}

/// What a run did (or, with `dry_run`, would do).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecretReport {
    pub repo: String,
    pub dry_run: bool,
    pub set: Vec<String>,
    pub skipped: Vec<Skip>,
}

/// Set the release secrets on `repo`. In a dry run nothing is executed and no
/// key is generated; the report shows what would happen.
pub fn bootstrap(
    ops: &dyn SecretOps,
    repo: &str,
    sources: &Sources,
    dry_run: bool,
) -> Result<SecretReport, ClihatchError> {
    let mut set = Vec::new();
    let mut skipped = Vec::new();

    match &sources.cargo_token {
        Some(token) => {
            if !dry_run {
                ops.set_secret(repo, "CARGO_REGISTRY_TOKEN", token)?;
            }
            set.push("CARGO_REGISTRY_TOKEN".to_string());
        }
        None => skipped.push(Skip {
            secret: "CARGO_REGISTRY_TOKEN".into(),
            reason: "no token in ~/.cargo/credentials.toml (run `cargo login`)".into(),
        }),
    }

    // Homebrew deploy key: the part worth automating.
    if !dry_run {
        let title = format!("{} release (CI push)", sources.crate_name);
        let (private_key, public_key) = ops.generate_keypair(&title)?;
        ops.add_deploy_key(&sources.tap, &title, &public_key)?;
        ops.set_secret(repo, "HOMEBREW_TAP_DEPLOY_KEY", &private_key)?;
    }
    set.push("HOMEBREW_TAP_DEPLOY_KEY".to_string());

    match &sources.pypi_token {
        Some(token) => {
            if !dry_run {
                ops.set_secret(repo, "PYPI_API_TOKEN", token)?;
            }
            set.push("PYPI_API_TOKEN".to_string());
        }
        None => skipped.push(Skip {
            secret: "PYPI_API_TOKEN".into(),
            reason: "set $PYPI_API_TOKEN, or pass --pypi-token-stdin".into(),
        }),
    }

    Ok(SecretReport {
        repo: repo.to_string(),
        dry_run,
        set,
        skipped,
    })
}

/// Extract the crates.io token from a Cargo `credentials.toml`. Handles both
/// `[registry]` and `[registries.crates-io]`; falls back to the first token
/// found if neither of those is present.
pub fn cargo_token_from_credentials(toml: &str) -> Option<String> {
    let mut section = String::new();
    let mut fallback = None;
    for raw in toml.lines() {
        let line = raw.trim();
        if let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = inner.trim().to_string();
            continue;
        }
        let Some(rest) = line.strip_prefix("token") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let value = rest.trim().trim_matches('"').to_string();
        if value.is_empty() {
            continue;
        }
        if section == "registry" || section == "registries.crates-io" {
            return Some(value);
        }
        fallback.get_or_insert(value);
    }
    fallback
}

/// The real backend: `gh` for secrets/deploy-keys, `ssh-keygen` for the key.
pub struct RealSecretOps;

fn backend(tool: &'static str, stderr: &[u8]) -> ClihatchError {
    ClihatchError::Backend {
        tool,
        message: String::from_utf8_lossy(stderr).trim().to_string(),
    }
}

impl SecretOps for RealSecretOps {
    fn set_secret(&self, repo: &str, name: &str, value: &str) -> Result<(), ClihatchError> {
        let mut child = Command::new("gh")
            .args(["secret", "set", name, "-R", repo])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ClihatchError::Backend {
                tool: "gh",
                message: e.to_string(),
            })?;
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(value.as_bytes())
            .map_err(|e| ClihatchError::Io {
                message: e.to_string(),
            })?;
        let out = child.wait_with_output().map_err(|e| ClihatchError::Io {
            message: e.to_string(),
        })?;
        if out.status.success() {
            Ok(())
        } else {
            Err(backend("gh", &out.stderr))
        }
    }

    fn generate_keypair(&self, comment: &str) -> Result<(String, String), ClihatchError> {
        let base = std::env::temp_dir().join(format!("clihatch-key-{}", std::process::id()));
        let pubp = base.with_file_name(format!(
            "{}.pub",
            base.file_name().unwrap().to_string_lossy()
        ));
        let _ = fs::remove_file(&base);
        let _ = fs::remove_file(&pubp);

        let out = Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-f",
                &base.to_string_lossy(),
                "-N",
                "",
                "-C",
                comment,
                "-q",
            ])
            .output()
            .map_err(|e| ClihatchError::Backend {
                tool: "ssh-keygen",
                message: e.to_string(),
            })?;
        if !out.status.success() {
            return Err(backend("ssh-keygen", &out.stderr));
        }
        let private_key = fs::read_to_string(&base).map_err(|e| ClihatchError::Io {
            message: e.to_string(),
        })?;
        let public_key = fs::read_to_string(&pubp).map_err(|e| ClihatchError::Io {
            message: e.to_string(),
        })?;
        let _ = fs::remove_file(&base);
        let _ = fs::remove_file(&pubp);
        Ok((private_key, public_key))
    }

    fn add_deploy_key(
        &self,
        tap: &str,
        title: &str,
        public_key: &str,
    ) -> Result<(), ClihatchError> {
        let path = std::env::temp_dir().join(format!("clihatch-pub-{}.pub", std::process::id()));
        fs::write(&path, public_key).map_err(|e| ClihatchError::Io {
            message: e.to_string(),
        })?;
        let out = Command::new("gh")
            .args([
                "repo",
                "deploy-key",
                "add",
                &path.to_string_lossy(),
                "-R",
                tap,
                "--allow-write",
                "--title",
                title,
            ])
            .output()
            .map_err(|e| ClihatchError::Backend {
                tool: "gh",
                message: e.to_string(),
            });
        let _ = fs::remove_file(&path);
        let out = out?;
        if out.status.success() {
            Ok(())
        } else {
            Err(backend("gh", &out.stderr))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeOps {
        secrets: RefCell<Vec<(String, String)>>,
        deploy_keys: RefCell<Vec<String>>,
    }
    impl SecretOps for FakeOps {
        fn set_secret(&self, _repo: &str, name: &str, value: &str) -> Result<(), ClihatchError> {
            self.secrets.borrow_mut().push((name.into(), value.into()));
            Ok(())
        }
        fn generate_keypair(&self, _c: &str) -> Result<(String, String), ClihatchError> {
            Ok(("PRIVATE".into(), "ssh-ed25519 AAAA...".into()))
        }
        fn add_deploy_key(&self, tap: &str, _t: &str, _k: &str) -> Result<(), ClihatchError> {
            self.deploy_keys.borrow_mut().push(tap.into());
            Ok(())
        }
    }

    fn sources(cargo: Option<&str>, pypi: Option<&str>) -> Sources {
        Sources {
            crate_name: "demo".into(),
            tap: "rvben/homebrew-tap".into(),
            cargo_token: cargo.map(String::from),
            pypi_token: pypi.map(String::from),
        }
    }

    #[test]
    fn sets_all_three_when_sources_present() {
        let ops = FakeOps::default();
        let report = bootstrap(
            &ops,
            "rvben/demo",
            &sources(Some("cio_x"), Some("pypi-y")),
            false,
        )
        .unwrap();
        assert_eq!(
            report.set,
            [
                "CARGO_REGISTRY_TOKEN",
                "HOMEBREW_TAP_DEPLOY_KEY",
                "PYPI_API_TOKEN"
            ]
        );
        assert!(report.skipped.is_empty());
        let names: Vec<String> = ops
            .secrets
            .borrow()
            .iter()
            .map(|(n, _)| n.clone())
            .collect();
        assert_eq!(
            names,
            [
                "CARGO_REGISTRY_TOKEN",
                "HOMEBREW_TAP_DEPLOY_KEY",
                "PYPI_API_TOKEN"
            ]
        );
        assert_eq!(*ops.deploy_keys.borrow(), ["rvben/homebrew-tap"]);
    }

    #[test]
    fn skips_missing_token_sources_but_still_does_homebrew() {
        let ops = FakeOps::default();
        let report = bootstrap(&ops, "rvben/demo", &sources(None, None), false).unwrap();
        assert_eq!(report.set, ["HOMEBREW_TAP_DEPLOY_KEY"]);
        assert_eq!(report.skipped.len(), 2);
    }

    #[test]
    fn dry_run_executes_nothing() {
        let ops = FakeOps::default();
        let report = bootstrap(&ops, "rvben/demo", &sources(Some("cio_x"), None), true).unwrap();
        assert!(report.dry_run);
        assert!(
            ops.secrets.borrow().is_empty(),
            "dry run must not set secrets"
        );
        assert!(
            ops.deploy_keys.borrow().is_empty(),
            "dry run must not register keys"
        );
        assert!(report.set.contains(&"HOMEBREW_TAP_DEPLOY_KEY".to_string()));
    }

    #[test]
    fn parses_cargo_token_from_registry_section() {
        let creds = "[registry]\ntoken = \"cio_abc123\"\n";
        assert_eq!(
            cargo_token_from_credentials(creds),
            Some("cio_abc123".into())
        );
    }

    #[test]
    fn parses_cargo_token_from_crates_io_section() {
        let creds = "[registries.crates-io]\ntoken = \"cio_xyz\"\n";
        assert_eq!(cargo_token_from_credentials(creds), Some("cio_xyz".into()));
    }

    #[test]
    fn no_cargo_token_when_absent() {
        assert_eq!(cargo_token_from_credentials("[other]\nkey = 1\n"), None);
    }

    /// The real backend's only network-free operation: generating the keypair.
    /// Verifies the `ssh-keygen` invocation, the `.pub` path handling, and that
    /// the temp files are cleaned up.
    #[test]
    fn real_generate_keypair_produces_an_ed25519_pair() {
        if Command::new("ssh-keygen").arg("-?").output().is_err() {
            eprintln!("skipping: ssh-keygen not available");
            return;
        }
        let (private_key, public_key) = RealSecretOps
            .generate_keypair("clihatch test key")
            .expect("ssh-keygen succeeds");
        assert!(
            private_key.contains("PRIVATE KEY"),
            "private key looks like a PEM: {private_key:.40}"
        );
        assert!(
            public_key.starts_with("ssh-ed25519 "),
            "public key is ed25519: {public_key:.40}"
        );
        assert!(
            public_key.contains("clihatch test key"),
            "public key carries the comment"
        );
        let leftover = std::env::temp_dir().join(format!("clihatch-key-{}", std::process::id()));
        assert!(!leftover.exists(), "private temp file removed");
        assert!(
            !leftover.with_extension("pub").exists(),
            "public temp file removed"
        );
    }
}
