//! The clispec v0.2 contract emitted by `clihatch schema`.

use serde_json::{Value, json};

/// The version of The CLI Spec this document conforms to.
pub const CLISPEC_VERSION: &str = "0.2";

/// Build the clispec contract as a JSON value.
pub fn contract() -> Value {
    json!({
        "clispec": CLISPEC_VERSION,
        "name": "clihatch",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "global_args": [
            {
                "name": "--output",
                "type": "string",
                "enum": ["auto", "json", "text"],
                "default": "auto",
                "description": "Output format. auto = text on a TTY, JSON when piped."
            }
        ],
        "commands": [
            {
                "name": "new",
                "description": "Scaffold a new clispec-compliant Rust CLI into ./<name>. Refuses if the directory exists.",
                "mutating": true,
                "stability": "stable",
                "args": [
                    {"name": "name", "type": "string", "required": true, "description": "Crate/binary name ([a-z][a-z0-9_-]*)."},
                    {"name": "--description", "type": "string", "required": false, "description": "One-line package description."},
                    {"name": "--owner", "type": "string", "required": false, "default": "rvben", "description": "GitHub owner for repo URLs."},
                    {"name": "--author", "type": "string", "required": false, "description": "Cargo/LICENSE author (default: git config)."},
                    {"name": "--into", "type": "path", "required": false, "default": ".", "description": "Directory to create the crate inside."},
                    {"name": "--no-git", "type": "boolean", "required": false, "default": false, "description": "Skip git init + initial commit."}
                ],
                "output_fields": [
                    {"name": "created", "type": "string", "description": "Path of the new crate directory."},
                    {"name": "files", "type": "string[]", "description": "Files written, relative to the crate."},
                    {"name": "committed", "type": "boolean"},
                    {"name": "next", "type": "string[]", "description": "Suggested next commands."}
                ]
            },
            {
                "name": "schema",
                "description": "Print this clispec contract as JSON.",
                "mutating": false,
                "stability": "stable"
            },
            {
                "name": "completions",
                "description": "Generate a shell completion script.",
                "mutating": false,
                "stability": "stable",
                "args": [
                    {"name": "shell", "type": "string", "required": true, "enum": ["bash", "zsh", "fish", "powershell", "elvish"], "description": "Target shell."}
                ]
            }
        ],
        "errors": [
            {"kind": "usage", "exit_code": 3, "retryable": false, "description": "Invalid command-line arguments or crate name."},
            {"kind": "exists", "exit_code": 3, "retryable": false, "description": "The target directory already exists."},
            {"kind": "io", "exit_code": 2, "retryable": false, "description": "A filesystem operation failed."},
            {"kind": "git", "exit_code": 2, "retryable": false, "description": "A git operation failed (the files were still written)."}
        ]
    })
}

/// The contract as a pretty-printed JSON string.
pub fn contract_json() -> String {
    serde_json::to_string_pretty(&contract()).expect("contract serializes")
}
