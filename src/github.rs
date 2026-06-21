//! Create the GitHub repo for a freshly scaffolded crate and push the initial
//! commit, so `clihatch secrets` and a release can run immediately.
//!
//! Shells out to `gh repo create`; the [`crate::process::CommandRunner`] seam
//! lets tests assert the exact invocation without spawning anything.

use std::path::Path;

use crate::error::ClihatchError;
use crate::process::CommandRunner;

/// Create a public GitHub repo `owner/name` from the scaffold in `dir`, set the
/// `origin` remote, and push. Returns the `owner/name` slug.
pub fn create_repo(
    runner: &dyn CommandRunner,
    owner: &str,
    name: &str,
    description: &str,
    dir: &Path,
) -> Result<String, ClihatchError> {
    let slug = format!("{owner}/{name}");
    let dir = dir.to_string_lossy().into_owned();
    let out = runner
        .run(
            "gh",
            &[
                "repo",
                "create",
                &slug,
                "--public",
                "--description",
                description,
                "--source",
                &dir,
                "--remote",
                "origin",
                "--push",
            ],
            None,
        )
        .map_err(|e| ClihatchError::backend("gh", e.to_string()))?;
    if out.success {
        Ok(slug)
    } else {
        Err(ClihatchError::backend("gh", out.stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::CmdOutput;
    use std::cell::RefCell;

    #[derive(Default)]
    struct RecordingRunner {
        calls: RefCell<Vec<Vec<String>>>,
        fail: Option<String>,
    }
    impl CommandRunner for RecordingRunner {
        fn run(
            &self,
            _program: &str,
            args: &[&str],
            _stdin: Option<&str>,
        ) -> std::io::Result<CmdOutput> {
            self.calls
                .borrow_mut()
                .push(args.iter().map(|s| s.to_string()).collect());
            Ok(match &self.fail {
                Some(stderr) => CmdOutput {
                    success: false,
                    stdout: String::new(),
                    stderr: stderr.clone(),
                },
                None => CmdOutput {
                    success: true,
                    stdout: String::new(),
                    stderr: String::new(),
                },
            })
        }
    }

    #[test]
    fn creates_a_public_repo_from_the_source_dir_and_pushes() {
        let runner = RecordingRunner::default();
        let slug = create_repo(
            &runner,
            "rvben",
            "demo",
            "a demo tool",
            Path::new("/tmp/demo"),
        )
        .unwrap();
        assert_eq!(slug, "rvben/demo");
        let args = &runner.calls.borrow()[0];
        assert_eq!(args[0..3], ["repo", "create", "rvben/demo"]);
        assert!(args.contains(&"--public".to_string()));
        assert!(args.contains(&"--push".to_string()));
        // --source points at the scaffold dir.
        let src = args.iter().position(|a| a == "--source").unwrap();
        assert_eq!(args[src + 1], "/tmp/demo");
        // description is passed through.
        let desc = args.iter().position(|a| a == "--description").unwrap();
        assert_eq!(args[desc + 1], "a demo tool");
    }

    #[test]
    fn surfaces_gh_failure_as_backend_error() {
        let runner = RecordingRunner {
            fail: Some("HTTP 422: name already exists".into()),
            ..Default::default()
        };
        let e = create_repo(&runner, "rvben", "demo", "d", Path::new(".")).unwrap_err();
        assert_eq!(e.kind(), "backend");
        assert!(e.to_string().contains("already exists"));
    }
}
