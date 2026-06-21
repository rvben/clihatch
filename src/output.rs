//! Rendering the scaffold outcome as text (TTY) or JSON (piped).

use crate::OutputFormat;
use crate::scaffold::Outcome;
use crate::secrets::SecretReport;
use serde_json::json;

/// Render what was created.
pub fn render(outcome: &Outcome, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => json!({
            "created": outcome.dir,
            "files": outcome.files,
            "committed": outcome.committed,
            "next": [format!("cd {}", outcome.dir), "make check".to_string()],
        })
        .to_string(),
        OutputFormat::Text => {
            let mut out = format!(
                "Scaffolded {} ({} files{})",
                outcome.dir,
                outcome.files.len(),
                if outcome.committed { ", committed" } else { "" }
            );
            out.push_str("\nNext: cd ");
            out.push_str(&outcome.dir);
            out.push_str(" && make check");
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
            out
        }
    }
}
