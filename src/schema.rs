//! The clispec v0.3 contract emitted by `clihatch schema`.

use serde_json::{Value, json};

/// The version of The CLI Spec this document conforms to.
pub const CLISPEC_VERSION: &str = "0.3";

/// Build the clispec contract as a JSON value.
pub fn contract() -> Value {
    json!({
        "clispec": CLISPEC_VERSION,
        "name": "clihatch",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "output": {"tty": "text", "piped": "json"},
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
                "effects": "non_idempotent",
                "mutating": true,
                "cardinality": "single",
                "stability": "stable",
                "args": [
                    {"name": "name", "type": "string", "required": true, "description": "Crate/binary name ([a-z][a-z0-9_-]*)."},
                    {"name": "--description", "type": "string", "required": false, "description": "One-line package description."},
                    {"name": "--owner", "type": "string", "required": false, "default": "rvben", "description": "GitHub owner for repo URLs."},
                    {"name": "--author", "type": "string", "required": false, "description": "Cargo/LICENSE author (default: git config)."},
                    {"name": "--into", "type": "path", "required": false, "default": ".", "description": "Directory to create the crate inside."},
                    {"name": "--no-git", "type": "boolean", "required": false, "default": false, "description": "Skip git init + initial commit."},
                    {"name": "--github", "type": "boolean", "required": false, "default": false, "description": "Also create the GitHub repo (owner/name) and push the initial commit. Requires git."},
                    {"name": "--no-pypi", "type": "boolean", "required": false, "default": false, "description": "Omit the PyPI/maturin pipeline; the crate publishes to crates.io + Homebrew only."}
                ],
                "output_fields": [
                    {"name": "created", "type": "string", "description": "Path of the new crate directory."},
                    {"name": "files", "type": "array", "items": {"type": "string"}, "description": "Files written, relative to the crate."},
                    {"name": "committed", "type": "boolean"},
                    {"name": "repo", "type": "string", "description": "The owner/name of the GitHub repo created with --github (null otherwise)."},
                    {"name": "next", "type": "array", "items": {"type": "string"}, "description": "Suggested next commands, through to a published release."}
                ]
            },
            {
                "name": "check",
                "description": "Validate a crate name and report whether its destination is available without writing anything.",
                "effects": "read_only",
                "mutating": false,
                "cardinality": "single",
                "args": [
                    {"name": "name", "type": "string", "required": true, "description": "Crate/binary name to validate."},
                    {"name": "--into", "type": "path", "required": false, "default": ".", "description": "Parent directory that would receive the crate."}
                ],
                "output_fields": [
                    {"name": "name", "type": "string"},
                    {"name": "destination", "type": "string"},
                    {"name": "available", "type": "boolean"}
                ],
                "example": {"args": ["check", "clispec-probe-do-not-create", "--into", "."]}
            },
            {
                "name": "secrets",
                "description": "Bootstrap a repo's release secrets: generate + register the Homebrew tap deploy key (rotating any prior key with the same title), and set CARGO_REGISTRY_TOKEN / PYPI_API_TOKEN from local sources. Preflights `gh` auth and repo access; missing token sources are skipped, not invented.",
                "effects": "non_idempotent",
                "mutating": true,
                "cardinality": "bounded",
                "stability": "stable",
                "args": [
                    {"name": "repo", "type": "string", "required": true, "description": "Target repo as owner/name, or a bare name (combined with --owner)."},
                    {"name": "--owner", "type": "string", "required": false, "default": "rvben", "description": "GitHub owner, used when repo is a bare name."},
                    {"name": "--tap", "type": "string", "required": false, "description": "Homebrew tap repo to register the deploy key on (default: <owner>/homebrew-tap, matching the generated release workflow)."},
                    {"name": "--pypi-token-stdin", "type": "boolean", "required": false, "default": false, "description": "Read the PyPI token from stdin (otherwise $PYPI_API_TOKEN/$UV_PUBLISH_TOKEN, then the [pypi] token in ~/.pypirc)."},
                    {"name": "--dry-run", "type": "boolean", "required": false, "default": false, "description": "Report what would be set without executing anything."},
                    {"name": "--verify", "type": "boolean", "required": false, "default": false, "description": "Read-only: report which release secrets are already set on the repo (no changes). Outputs present/missing instead of set/skipped."}
                ],
                "output_fields": [
                    {"name": "repo", "type": "string", "description": "Resolved owner/name the secrets target."},
                    {"name": "dry_run", "type": "boolean"},
                    {"name": "set", "type": "array", "items": {"type": "string"}, "description": "Secret names set (or, in a dry run, that would be set)."},
                    {"name": "skipped", "type": "array", "items": {"type": "object"}, "description": "Secrets not set, each with `secret` and `reason`."},
                    {"name": "notes", "type": "array", "items": {"type": "string"}, "description": "Side notes worth surfacing, e.g. a rotated deploy key."},
                    {"name": "present", "type": "array", "items": {"type": "string"}, "description": "With --verify: release secrets already set on the repo."},
                    {"name": "missing", "type": "array", "items": {"type": "string"}, "description": "With --verify: release secrets not yet set."}
                ]
            },
            {
                "name": "schema",
                "description": "Print this clispec contract as JSON.",
                "effects": "read_only",
                "mutating": false,
                "cardinality": "single",
                "stability": "stable",
                "stdout_schema": {"$ref": "https://clispec.dev/schema/v0.3.json"}
            },
            {
                "name": "completions",
                "description": "Generate a shell completion script.",
                "effects": "read_only",
                "mutating": false,
                "output_kind": "opaque",
                "media_type": "text/plain",
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
            {"kind": "git", "exit_code": 2, "retryable": false, "description": "A git operation failed (the files were still written)."},
            {"kind": "backend", "exit_code": 2, "retryable": false, "description": "An external tool (gh, ssh-keygen) failed; check `gh auth status`."}
        ]
    })
}

/// The contract as a pretty-printed JSON string.
pub fn contract_json() -> String {
    serde_json::to_string_pretty(&contract()).expect("contract serializes")
}
