//! Rendering the scaffold outcome as text (TTY) or JSON (piped).

use crate::OutputFormat;
use crate::scaffold::Outcome;
use crate::secrets::SecretReport;
use serde_json::json;

/// The suggested next steps, covering the path to a published release. When the
/// repo was already created (`--github`), the "create the repo" step is omitted.
pub fn next_steps(outcome: &Outcome) -> Vec<String> {
    let mut steps = vec![format!("cd {} && make check", outcome.dir)];
    if outcome.repo.is_none() {
        steps.push(format!(
            "create the GitHub repo + push (or re-run with --github): \
             gh repo create <owner>/{} --source . --push",
            outcome.name
        ));
    }
    steps.push(format!("clihatch secrets {}", outcome.name));
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
