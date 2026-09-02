//! Bootstrap a repo's remaining release secrets:
//! - `HOMEBREW_TAP_DEPLOY_KEY` - generate an ed25519 key, register it as a
//!   write deploy key on the tap (rotating any prior key with the same title),
//!   and store the private key (fully automated);
//! - `PYPI_API_TOKEN` - from the environment / stdin.
//!
//! crates.io authentication is intentionally omitted: generated workflows use
//! GitHub OIDC Trusted Publishing and do not need a long-lived repository secret.
//!
//! Two seams keep this testable offline:
//! - [`SecretOps`] - the orchestration substitutes a fake backend;
//! - [`crate::process::CommandRunner`] - the real backend's `gh` / `ssh-keygen`
//!   invocations are recorded and asserted without spawning anything.

use std::fs;

use serde::{Deserialize, Serialize};

use crate::error::ClihatchError;
use crate::process::{CmdOutput, CommandRunner, SystemRunner};

/// External side effects a secrets run performs.
pub trait SecretOps {
    /// Verify prerequisites (auth, repo access) before any mutation.
    fn preflight(&self, _repo: &str) -> Result<(), ClihatchError> {
        Ok(())
    }
    fn set_secret(&self, repo: &str, name: &str, value: &str) -> Result<(), ClihatchError>;
    /// Generate an ed25519 keypair, returning `(private_key, public_key)`.
    fn generate_keypair(&self, comment: &str) -> Result<(String, String), ClihatchError>;
    /// Register `public_key` as a write deploy key on `tap`, first removing any
    /// existing key with the same `title`. Returns `true` if a key was rotated.
    fn add_deploy_key(
        &self,
        tap: &str,
        title: &str,
        public_key: &str,
    ) -> Result<bool, ClihatchError>;
    /// List the names of the secrets currently set on `repo`.
    fn list_secrets(&self, repo: &str) -> Result<Vec<String>, ClihatchError>;
}

/// The release secrets this tool bootstraps, in publish order.
pub const RELEASE_SECRETS: [&str; 2] = ["HOMEBREW_TAP_DEPLOY_KEY", "PYPI_API_TOKEN"];

/// The result of `secrets --verify`: which release secrets are set on the repo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerifyReport {
    pub repo: String,
    pub present: Vec<String>,
    pub missing: Vec<String>,
}

/// Read-only check of which release secrets are already set on `repo`.
pub fn verify(ops: &dyn SecretOps, repo: &str) -> Result<VerifyReport, ClihatchError> {
    let existing = ops.list_secrets(repo)?;
    let (present, missing) = RELEASE_SECRETS
        .iter()
        .map(|s| s.to_string())
        .partition(|s| existing.iter().any(|e| e == s));
    Ok(VerifyReport {
        repo: repo.to_string(),
        present,
        missing,
    })
}

/// The tokens and targets a run draws on.
#[derive(Debug, Clone)]
pub struct Sources {
    pub crate_name: String,
    pub tap: String,
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
    /// Side notes worth surfacing (e.g. a rotated deploy key).
    pub notes: Vec<String>,
}

/// Set the release secrets on `repo`. In a dry run nothing is executed, no key
/// is generated, and no preflight runs; the report shows what would happen.
pub fn bootstrap(
    ops: &dyn SecretOps,
    repo: &str,
    sources: &Sources,
    dry_run: bool,
) -> Result<SecretReport, ClihatchError> {
    let mut set = Vec::new();
    let mut skipped = Vec::new();
    let mut notes = Vec::new();

    if !dry_run {
        ops.preflight(repo)?;
    }

    // Homebrew deploy key: the part worth automating.
    if !dry_run {
        let title = format!("{} release (CI push)", sources.crate_name);
        let (private_key, public_key) = ops.generate_keypair(&title)?;
        if ops.add_deploy_key(&sources.tap, &title, &public_key)? {
            notes.push(format!(
                "rotated existing deploy key {title:?} on {}",
                sources.tap
            ));
        }
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
            reason: "no token in $PYPI_API_TOKEN or ~/.pypirc (or pass --pypi-token-stdin)".into(),
        }),
    }

    Ok(SecretReport {
        repo: repo.to_string(),
        dry_run,
        set,
        skipped,
        notes,
    })
}

/// Extract the PyPI upload token from a `.pypirc`: the `password` under the
/// `[pypi]` section (the conventional `__token__` password). The `[testpypi]`
/// and `[distutils]` sections are ignored.
pub fn pypi_token_from_pypirc(pypirc: &str) -> Option<String> {
    let mut in_pypi = false;
    for raw in pypirc.lines() {
        let line = raw.trim();
        if let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_pypi = inner.trim() == "pypi";
            continue;
        }
        if !in_pypi {
            continue;
        }
        if let Some(rest) = line.strip_prefix("password")
            && let Some(value) = rest.trim_start().strip_prefix('=')
        {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// The real backend: `gh` for secrets/deploy-keys, `ssh-keygen` for the key.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealSecretOps<R = SystemRunner> {
    runner: R,
}

impl RealSecretOps<SystemRunner> {
    pub fn new() -> Self {
        Self {
            runner: SystemRunner,
        }
    }
}

impl<R: CommandRunner> RealSecretOps<R> {
    /// Build a backend over a specific command runner (used by tests).
    pub fn with_runner(runner: R) -> Self {
        Self { runner }
    }

    fn gh(&self, args: &[&str]) -> Result<CmdOutput, ClihatchError> {
        self.runner
            .run("gh", args, None)
            .map_err(|e| ClihatchError::backend("gh", e.to_string()))
    }
}

/// One deploy key as reported by `gh repo deploy-key list --json id,title`.
#[derive(Debug, Deserialize)]
struct DeployKey {
    id: i64,
    title: String,
}

impl<R: CommandRunner> SecretOps for RealSecretOps<R> {
    fn preflight(&self, repo: &str) -> Result<(), ClihatchError> {
        let auth = self.gh(&["auth", "status"])?;
        if !auth.success {
            return Err(ClihatchError::Backend {
                tool: "gh",
                message: "not authenticated; run `gh auth login`".into(),
            });
        }
        let view = self.gh(&["repo", "view", repo, "--json", "name"])?;
        if !view.success {
            return Err(ClihatchError::Backend {
                tool: "gh",
                message: format!(
                    "repo {repo} not found or inaccessible: {}",
                    view.stderr.trim()
                ),
            });
        }
        Ok(())
    }

    fn set_secret(&self, repo: &str, name: &str, value: &str) -> Result<(), ClihatchError> {
        let out = self
            .runner
            .run("gh", &["secret", "set", name, "-R", repo], Some(value))
            .map_err(|e| ClihatchError::backend("gh", e.to_string()))?;
        if out.success {
            Ok(())
        } else {
            Err(ClihatchError::backend("gh", out.stderr))
        }
    }

    fn generate_keypair(&self, comment: &str) -> Result<(String, String), ClihatchError> {
        let base = std::env::temp_dir().join(format!("clihatch-key-{}", std::process::id()));
        let pubp = base.with_file_name(format!(
            "{}.pub",
            base.file_name().expect("temp file name").to_string_lossy()
        ));
        let _ = fs::remove_file(&base);
        let _ = fs::remove_file(&pubp);

        let base_str = base.to_string_lossy().into_owned();
        let out = self
            .runner
            .run(
                "ssh-keygen",
                &[
                    "-t", "ed25519", "-f", &base_str, "-N", "", "-C", comment, "-q",
                ],
                None,
            )
            .map_err(|e| ClihatchError::backend("ssh-keygen", e.to_string()))?;
        if !out.success {
            return Err(ClihatchError::backend("ssh-keygen", out.stderr));
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
    ) -> Result<bool, ClihatchError> {
        // Rotate: drop any existing key with this title so the stored secret and
        // the registered key always match.
        let listed = self.gh(&[
            "repo",
            "deploy-key",
            "list",
            "-R",
            tap,
            "--json",
            "id,title",
        ])?;
        if !listed.success {
            return Err(ClihatchError::backend("gh", listed.stderr));
        }
        let keys: Vec<DeployKey> = serde_json::from_str(&listed.stdout)
            .map_err(|e| ClihatchError::backend("gh", format!("parsing deploy-key list: {e}")))?;
        let mut rotated = false;
        for key in keys.iter().filter(|k| k.title == title) {
            let id = key.id.to_string();
            let del = self.gh(&["repo", "deploy-key", "delete", &id, "-R", tap])?;
            if !del.success {
                return Err(ClihatchError::backend("gh", del.stderr));
            }
            rotated = true;
        }

        let path = std::env::temp_dir().join(format!("clihatch-pub-{}.pub", std::process::id()));
        fs::write(&path, public_key).map_err(|e| ClihatchError::Io {
            message: e.to_string(),
        })?;
        let path_str = path.to_string_lossy().into_owned();
        let added = self.gh(&[
            "repo",
            "deploy-key",
            "add",
            &path_str,
            "-R",
            tap,
            "--allow-write",
            "--title",
            title,
        ]);
        let _ = fs::remove_file(&path);
        let added = added?;
        if added.success {
            Ok(rotated)
        } else {
            Err(ClihatchError::backend("gh", added.stderr))
        }
    }

    fn list_secrets(&self, repo: &str) -> Result<Vec<String>, ClihatchError> {
        let out = self.gh(&[
            "secret", "list", "-R", repo, "--json", "name", "-q", ".[].name",
        ])?;
        if !out.success {
            return Err(ClihatchError::backend("gh", out.stderr));
        }
        Ok(out
            .stdout
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

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
        fn add_deploy_key(&self, tap: &str, _t: &str, _k: &str) -> Result<bool, ClihatchError> {
            self.deploy_keys.borrow_mut().push(tap.into());
            Ok(false)
        }
        fn list_secrets(&self, _repo: &str) -> Result<Vec<String>, ClihatchError> {
            Ok(Vec::new())
        }
    }

    fn sources(pypi: Option<&str>) -> Sources {
        Sources {
            crate_name: "demo".into(),
            tap: "rvben/homebrew-tap".into(),
            pypi_token: pypi.map(String::from),
        }
    }

    #[test]
    fn sets_both_when_sources_present() {
        let ops = FakeOps::default();
        let report = bootstrap(&ops, "rvben/demo", &sources(Some("pypi-y")), false).unwrap();
        assert_eq!(report.set, ["HOMEBREW_TAP_DEPLOY_KEY", "PYPI_API_TOKEN"]);
        assert!(report.skipped.is_empty());
        let names: Vec<String> = ops
            .secrets
            .borrow()
            .iter()
            .map(|(n, _)| n.clone())
            .collect();
        assert_eq!(names, ["HOMEBREW_TAP_DEPLOY_KEY", "PYPI_API_TOKEN"]);
        assert_eq!(*ops.deploy_keys.borrow(), ["rvben/homebrew-tap"]);
    }

    #[test]
    fn skips_missing_token_sources_but_still_does_homebrew() {
        let ops = FakeOps::default();
        let report = bootstrap(&ops, "rvben/demo", &sources(None), false).unwrap();
        assert_eq!(report.set, ["HOMEBREW_TAP_DEPLOY_KEY"]);
        assert_eq!(report.skipped.len(), 1);
    }

    #[test]
    fn dry_run_executes_nothing() {
        let ops = FakeOps::default();
        let report = bootstrap(&ops, "rvben/demo", &sources(None), true).unwrap();
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
    fn reads_pypi_token_from_pypi_section() {
        let rc = "[pypi]\nusername = __token__\npassword = pypi-AgEN123\n";
        assert_eq!(pypi_token_from_pypirc(rc), Some("pypi-AgEN123".into()));
    }

    #[test]
    fn ignores_testpypi_section() {
        let rc = "[testpypi]\nusername = __token__\npassword = pypi-TEST\n";
        assert_eq!(pypi_token_from_pypirc(rc), None);
    }

    #[test]
    fn prefers_pypi_over_other_sections() {
        let rc = "[distutils]\nindex-servers = pypi\n\n[testpypi]\npassword = pypi-WRONG\n\n[pypi]\nusername = __token__\npassword = pypi-RIGHT\n";
        assert_eq!(pypi_token_from_pypirc(rc), Some("pypi-RIGHT".into()));
    }

    /// One recorded call: (program, args, stdin).
    type Call = (String, Vec<String>, Option<String>);
    /// A call's (args, stdin) once filtered to a single program.
    type Invocation = (Vec<String>, Option<String>);

    /// A recording runner: captures every [`Call`] and returns queued responses
    /// (defaulting to empty success). Used to assert the exact `gh` invocations;
    /// `ssh-keygen` is covered by the real backend test.
    #[derive(Default)]
    struct RecordingRunner {
        calls: RefCell<Vec<Call>>,
        responses: RefCell<VecDeque<CmdOutput>>,
    }
    impl RecordingRunner {
        fn with(responses: Vec<CmdOutput>) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                responses: RefCell::new(responses.into()),
            }
        }
        fn calls_to(&self, program: &str) -> Vec<Invocation> {
            self.calls
                .borrow()
                .iter()
                .filter(|(p, ..)| p == program)
                .map(|(_, a, s)| (a.clone(), s.clone()))
                .collect()
        }
    }
    impl CommandRunner for RecordingRunner {
        fn run(
            &self,
            program: &str,
            args: &[&str],
            stdin: Option<&str>,
        ) -> std::io::Result<CmdOutput> {
            self.calls.borrow_mut().push((
                program.to_string(),
                args.iter().map(|s| s.to_string()).collect(),
                stdin.map(String::from),
            ));
            Ok(self
                .responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(CmdOutput {
                    success: true,
                    stdout: String::new(),
                    stderr: String::new(),
                }))
        }
    }

    fn ok(stdout: &str) -> CmdOutput {
        CmdOutput {
            success: true,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }
    fn err(stderr: &str) -> CmdOutput {
        CmdOutput {
            success: false,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }

    #[test]
    fn set_secret_pipes_value_on_stdin_with_correct_argv() {
        let ops = RealSecretOps::with_runner(RecordingRunner::default());
        ops.set_secret("rvben/demo", "PYPI_API_TOKEN", "pypi-secret")
            .unwrap();
        let calls = ops.runner.calls_to("gh");
        assert_eq!(
            calls,
            [(
                vec![
                    "secret".into(),
                    "set".into(),
                    "PYPI_API_TOKEN".into(),
                    "-R".into(),
                    "rvben/demo".into()
                ],
                Some("pypi-secret".into())
            )]
        );
    }

    #[test]
    fn set_secret_surfaces_gh_failure_as_backend_error() {
        let ops = RealSecretOps::with_runner(RecordingRunner::with(vec![err("HTTP 404")]));
        let e = ops.set_secret("rvben/demo", "X", "v").unwrap_err();
        assert_eq!(e.kind(), "backend");
        assert!(e.to_string().contains("HTTP 404"));
    }

    #[test]
    fn add_deploy_key_rotates_matching_title_then_adds() {
        // list returns one matching + one unrelated key; delete + add succeed.
        let runner = RecordingRunner::with(vec![
            ok(r#"[{"id":42,"title":"demo release (CI push)"},{"id":7,"title":"other"}]"#),
            ok(""), // delete 42
            ok(""), // add
        ]);
        let ops = RealSecretOps::with_runner(runner);
        let rotated = ops
            .add_deploy_key(
                "rvben/homebrew-tap",
                "demo release (CI push)",
                "ssh-ed25519 K",
            )
            .unwrap();
        assert!(rotated, "an existing key with the title was rotated");
        let calls = ops.runner.calls_to("gh");
        // list, delete 42, add - the unrelated key 7 is untouched.
        assert_eq!(calls[0].0[..3], ["repo", "deploy-key", "list"]);
        assert_eq!(
            calls[1].0,
            [
                "repo",
                "deploy-key",
                "delete",
                "42",
                "-R",
                "rvben/homebrew-tap"
            ]
        );
        assert_eq!(calls[2].0[..3], ["repo", "deploy-key", "add"]);
        assert!(calls[2].0.contains(&"--allow-write".to_string()));
        assert!(calls.iter().all(|(a, _)| !a.contains(&"7".to_string())));
    }

    #[test]
    fn add_deploy_key_no_rotation_when_title_absent() {
        let runner = RecordingRunner::with(vec![ok("[]"), ok("")]);
        let ops = RealSecretOps::with_runner(runner);
        let rotated = ops
            .add_deploy_key(
                "rvben/homebrew-tap",
                "demo release (CI push)",
                "ssh-ed25519 K",
            )
            .unwrap();
        assert!(!rotated);
        let calls = ops.runner.calls_to("gh");
        assert_eq!(calls.len(), 2, "list + add only, no delete");
    }

    #[test]
    fn preflight_fails_fast_when_not_authenticated() {
        let runner = RecordingRunner::with(vec![err("You are not logged into any GitHub hosts")]);
        let ops = RealSecretOps::with_runner(runner);
        let e = ops.preflight("rvben/demo").unwrap_err();
        assert_eq!(e.kind(), "backend");
        assert!(e.to_string().contains("not authenticated"));
        // It stops at auth - never reaches `repo view`.
        assert_eq!(ops.runner.calls_to("gh").len(), 1);
    }

    #[test]
    fn bootstrap_preflight_aborts_before_any_mutation() {
        let runner = RecordingRunner::with(vec![err("not logged in")]);
        let ops = RealSecretOps::with_runner(runner);
        let e = bootstrap(&ops, "rvben/demo", &sources(None), false).unwrap_err();
        assert_eq!(e.kind(), "backend");
        // Only the auth probe ran; no secret was set.
        assert_eq!(ops.runner.calls_to("gh").len(), 1);
    }

    #[test]
    fn verify_partitions_present_and_missing_release_secrets() {
        // `gh secret list` returns one of the two release secrets.
        let runner = RecordingRunner::with(vec![ok("HOMEBREW_TAP_DEPLOY_KEY\n")]);
        let ops = RealSecretOps::with_runner(runner);
        let report = verify(&ops, "rvben/demo").unwrap();
        assert_eq!(report.present, ["HOMEBREW_TAP_DEPLOY_KEY"]);
        assert_eq!(report.missing, ["PYPI_API_TOKEN"]);
        // Read-only argv: gh secret list -R <repo> --json name -q .[].name
        let calls = ops.runner.calls_to("gh");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0[0..4], ["secret", "list", "-R", "rvben/demo"]);
        assert!(calls[0].0.contains(&"--json".to_string()));
    }

    /// The real backend's only network-free operation: generating the keypair.
    /// Verifies the `ssh-keygen` invocation, the `.pub` path handling, and that
    /// the temp files are cleaned up.
    #[test]
    fn real_generate_keypair_produces_an_ed25519_pair() {
        if std::process::Command::new("ssh-keygen")
            .arg("-?")
            .output()
            .is_err()
        {
            eprintln!("skipping: ssh-keygen not available");
            return;
        }
        let (private_key, public_key) = RealSecretOps::new()
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
