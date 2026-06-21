//! clihatch CLI: scaffold a clispec-compliant Rust CLI.
//!
//! Follows The CLI Spec (clispec.dev): text on a TTY, JSON when piped,
//! structured error envelopes on the last line of stderr, a `schema`
//! subcommand. `new` is the one `mutating: true` command.

use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::{Command as PCommand, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::error::ErrorKind as ClapErrorKind;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clihatch::{
    ClihatchError, OutputFormat, RealSecretOps, Request, Sources, bootstrap,
    cargo_token_from_credentials, render, render_secrets, run, schema,
};
use serde_json::json;

#[derive(Parser)]
#[command(
    name = "clihatch",
    version,
    about = "Scaffold a clispec-compliant, agent-facing Rust CLI in seconds.",
    long_about = "Scaffold a complete, clispec-compliant Rust CLI: source skeleton, schema + \
                  conformance test, and the GitHub-hosted dual-publish release pipeline.\n\n\
                  Run `clihatch schema` for the machine-readable contract (clispec.dev).",
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Output format; auto = text on a TTY, JSON when piped.
    #[arg(long, short = 'o', value_enum, default_value = "auto", global = true)]
    output: CliOutput,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a new clispec-compliant Rust CLI into ./<name>.
    New {
        /// Crate/binary name ([a-z][a-z0-9_-]*).
        name: String,
        /// One-line package description.
        #[arg(long)]
        description: Option<String>,
        /// GitHub owner for repo URLs.
        #[arg(long, default_value = "rvben")]
        owner: String,
        /// Cargo/LICENSE author (default: from git config).
        #[arg(long)]
        author: Option<String>,
        /// Directory to create the crate inside.
        #[arg(long, default_value = ".")]
        into: PathBuf,
        /// Skip git init + initial commit.
        #[arg(long)]
        no_git: bool,
    },
    /// Bootstrap a repo's release secrets (Homebrew deploy key, cargo + PyPI tokens).
    Secrets {
        /// Target repo as owner/name, or a bare name (combined with --owner).
        repo: String,
        /// GitHub owner, used when repo is a bare name.
        #[arg(long, default_value = "rvben")]
        owner: String,
        /// Homebrew tap repo to register the deploy key on.
        #[arg(long, default_value = "rvben/homebrew-tap")]
        tap: String,
        /// Read the PyPI token from stdin instead of $PYPI_API_TOKEN.
        #[arg(long)]
        pypi_token_stdin: bool,
        /// Report what would be set without executing anything.
        #[arg(long)]
        dry_run: bool,
    },
    /// Print the machine-readable contract (clispec.dev) as JSON.
    Schema,
    /// Generate a shell completion script.
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CliOutput {
    Auto,
    Json,
    Text,
}

impl CliOutput {
    fn resolve(self) -> OutputFormat {
        match self {
            CliOutput::Json => OutputFormat::Json,
            CliOutput::Text => OutputFormat::Text,
            CliOutput::Auto => {
                if std::io::stdout().is_terminal() {
                    OutputFormat::Text
                } else {
                    OutputFormat::Json
                }
            }
        }
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => return handle_clap_error(e),
    };
    let format = cli.output.resolve();

    match cli.command {
        Some(Command::Schema) => {
            println!("{}", schema::contract_json());
            ExitCode::SUCCESS
        }
        Some(Command::Completions { shell }) => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
            ExitCode::SUCCESS
        }
        Some(Command::New {
            name,
            description,
            owner,
            author,
            into,
            no_git,
        }) => {
            let request = Request {
                description: description
                    .unwrap_or_else(|| format!("A clispec-compliant CLI ({name}).")),
                author: author.unwrap_or_else(default_author),
                year: current_year(),
                name,
                owner,
                into,
                git: !no_git,
            };
            match run(&request) {
                Ok(outcome) => {
                    let _ = writeln!(std::io::stdout(), "{}", render(&outcome, format));
                    ExitCode::SUCCESS
                }
                Err(err) => fail(&err),
            }
        }
        Some(Command::Secrets {
            repo,
            owner,
            tap,
            pypi_token_stdin,
            dry_run,
        }) => {
            let full_repo = if repo.contains('/') {
                repo.clone()
            } else {
                format!("{owner}/{repo}")
            };
            let crate_name = full_repo
                .rsplit('/')
                .next()
                .unwrap_or(&full_repo)
                .to_string();
            let pypi_token = match read_pypi_token(pypi_token_stdin) {
                Ok(token) => token,
                Err(err) => return fail(&err),
            };
            let sources = Sources {
                crate_name,
                tap,
                cargo_token: read_cargo_token(),
                pypi_token,
            };
            match bootstrap(&RealSecretOps, &full_repo, &sources, dry_run) {
                Ok(report) => {
                    let _ = writeln!(std::io::stdout(), "{}", render_secrets(&report, format));
                    ExitCode::SUCCESS
                }
                Err(err) => fail(&err),
            }
        }
        None => {
            let err = ClihatchError::Usage {
                message: "no command given (try `clihatch new <name>`)".into(),
            };
            fail(&err)
        }
    }
}

/// Read the crates.io token from `$CARGO_REGISTRY_TOKEN`, else from the Cargo
/// credentials file under `$CARGO_HOME` (or `~/.cargo`).
fn read_cargo_token() -> Option<String> {
    if let Ok(token) = std::env::var("CARGO_REGISTRY_TOKEN") {
        let token = token.trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }
    let cargo_home = std::env::var("CARGO_HOME")
        .unwrap_or_else(|_| format!("{}/.cargo", std::env::var("HOME").unwrap_or_default()));
    let dir = PathBuf::from(cargo_home);
    let contents = std::fs::read_to_string(dir.join("credentials.toml"))
        .or_else(|_| std::fs::read_to_string(dir.join("credentials")))
        .ok()?;
    cargo_token_from_credentials(&contents)
}

/// Resolve the PyPI token: stdin when requested, else `$PYPI_API_TOKEN` /
/// `$UV_PUBLISH_TOKEN`. Absent is fine - the secret is skipped, not invented.
fn read_pypi_token(from_stdin: bool) -> Result<Option<String>, ClihatchError> {
    if from_stdin {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| ClihatchError::Io {
                message: e.to_string(),
            })?;
        let buf = buf.trim().to_string();
        return Ok((!buf.is_empty()).then_some(buf));
    }
    for key in ["PYPI_API_TOKEN", "UV_PUBLISH_TOKEN"] {
        if let Ok(token) = std::env::var(key) {
            let token = token.trim();
            if !token.is_empty() {
                return Ok(Some(token.to_string()));
            }
        }
    }
    Ok(None)
}

/// Author string from `git config`, falling back to a placeholder.
fn default_author() -> String {
    let get = |key: &str| {
        PCommand::new("git")
            .args(["config", key])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    match (get("user.name"), get("user.email")) {
        (Some(name), Some(email)) => format!("{name} <{email}>"),
        (Some(name), None) => name,
        _ => "Your Name <you@example.com>".to_string(),
    }
}

/// Current year (UTC, good enough for a copyright line) without a date crate.
fn current_year() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Average Gregorian year length accounts for leap years.
    (1970 + secs / 31_556_952).to_string()
}

fn fail(err: &ClihatchError) -> ExitCode {
    emit_error(err);
    ExitCode::from(err.exit_code() as u8)
}

/// Help and version print normally and exit 0; every other clap failure becomes
/// a structured `usage` error envelope.
fn handle_clap_error(e: clap::Error) -> ExitCode {
    match e.kind() {
        ClapErrorKind::DisplayHelp
        | ClapErrorKind::DisplayVersion
        | ClapErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            let _ = e.print();
            ExitCode::SUCCESS
        }
        _ => fail(&ClihatchError::Usage {
            message: e.to_string().trim().to_string(),
        }),
    }
}

/// Write the clispec error envelope as the last line of stderr.
fn emit_error(err: &ClihatchError) {
    let mut error = serde_json::Map::new();
    error.insert("kind".into(), json!(err.kind()));
    error.insert("message".into(), json!(err.to_string()));
    error.insert("exit_code".into(), json!(err.exit_code()));
    if let Some(hint) = err.hint() {
        error.insert("hint".into(), json!(hint));
    }
    eprintln!("{}", json!({ "error": error }));
}
