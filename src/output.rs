//! Rendering the scaffold outcome as text (TTY) or JSON (piped).

use crate::OutputFormat;
use crate::scaffold::Outcome;
use crate::secrets::{SecretReport, VerifyReport};
use serde_json::json;

/// The suggested next steps, covering the path to a published release. When the
/// repo was already created (`--github`), the "create the repo" step is omitted.
/// The configured owner is carried through, so the commands target the right
/// repo even with a non-default `--owner`.
pub fn next_steps(outcome: &Outcome) -> Vec<String> {
    let slug = format!("{}/{}", outcome.owner, outcome.name);
    let mut steps = vec![format!("cd {} && make check", outcome.dir)];
    if outcome.repo.is_none() {
        steps.push(format!(
            "create the GitHub repo + push (or re-run with --github): \
             gh repo create {slug} --source . --push"
        ));
    }
    steps.push(format!("clihatch secrets {slug}"));
    steps.push("vership release  # tag + dual-publish".to_string());
    steps
}

/// Render what was created.
pub fn render(outcome: &Outcome, format: OutputFormat) -> String {
    let next = next_steps(outcome);
    match format {
        OutputFormat::Json => json!({
            "created": outcome.dir,
            "files": outcome.files,
            "committed": outcome.committed,
            "repo": outcome.repo,
            "next": next,
        })
        .to_string(),
        OutputFormat::Text => {
            let mut out = format!(
                "Scaffolded {} ({} files{})",
                outcome.dir,
                outcome.files.len(),
                if outcome.committed { ", committed" } else { "" }
            );
            if let Some(repo) = &outcome.repo {
                out.push_str(&format!("\nCreated and pushed github.com/{repo}"));
            }
            out.push_str("\nNext:");
            for step in &next {
                out.push_str(&format!("\n  - {step}"));
            }
            out
        }
    }
}

/// Render a secrets-bootstrap report as text (TTY) or JSON (piped).
pub fn render_secrets(report: &SecretReport, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string(report).expect("report serializes"),
        OutputFormat::Text => {
            let verb = if report.dry_run { "would set" } else { "set" };
            let mut out = format!("Secrets on {}", report.repo);
            if report.dry_run {
                out.push_str(" (dry run)");
            }
            if !report.set.is_empty() {
                out.push_str(&format!("\n  {}: {}", verb, report.set.join(", ")));
            }
            for skip in &report.skipped {
                out.push_str(&format!("\n  skipped {}: {}", skip.secret, skip.reason));
            }
            for note in &report.notes {
                out.push_str(&format!("\n  note: {note}"));
            }
            out
        }
    }
}

/// Render a `secrets --verify` report as text (TTY) or JSON (piped).
pub fn render_verify(report: &VerifyReport, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string(report).expect("report serializes"),
        OutputFormat::Text => {
            let mut out = format!("Release secrets on {}", report.repo);
            if !report.present.is_empty() {
                out.push_str(&format!("\n  set: {}", report.present.join(", ")));
            }
            if !report.missing.is_empty() {
                out.push_str(&format!("\n  missing: {}", report.missing.join(", ")));
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scaffold::Outcome;

    fn outcome(owner: &str, repo: Option<&str>) -> Outcome {
        Outcome {
            name: "demo".into(),
            owner: owner.into(),
            dir: "./demo".into(),
            files: vec![],
            committed: true,
            repo: repo.map(String::from),
        }
    }

    #[test]
    fn next_steps_carry_non_default_owner() {
        let steps = next_steps(&outcome("acme", None));
        assert!(
            steps.iter().any(|s| s.contains("gh repo create acme/demo")),
            "create step targets the configured owner: {steps:?}"
        );
        assert!(
            steps.iter().any(|s| s == "clihatch secrets acme/demo"),
            "secrets step targets owner/name, not the default owner: {steps:?}"
        );
    }

    #[test]
    fn next_steps_omit_create_when_repo_exists() {
        let steps = next_steps(&outcome("rvben", Some("rvben/demo")));
        assert!(!steps.iter().any(|s| s.contains("gh repo create")));
        assert!(steps.iter().any(|s| s == "clihatch secrets rvben/demo"));
    }
}
