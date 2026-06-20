//! Rendering the scaffold outcome as text (TTY) or JSON (piped).

use crate::OutputFormat;
use crate::scaffold::Outcome;
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
