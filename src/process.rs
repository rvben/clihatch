//! The external-command seam, shared by the `secrets` and `github` backends.
//!
//! Tests substitute a recording runner and assert the exact argv and stdin, so
//! the `gh` / `ssh-keygen` invocations are verified without spawning anything.

use std::io::Write;
use std::process::{Command, Stdio};

/// The result of running an external command.
#[derive(Debug, Clone)]
pub struct CmdOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Runs external commands.
pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str], stdin: Option<&str>) -> std::io::Result<CmdOutput>;
}

/// Runs commands as real subprocesses.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &str, args: &[&str], stdin: Option<&str>) -> std::io::Result<CmdOutput> {
        let mut cmd = Command::new(program);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd.stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        let mut child = cmd.spawn()?;
        if let Some(data) = stdin {
            child
                .stdin
                .take()
                .expect("piped stdin")
                .write_all(data.as_bytes())?;
        }
        let out = child.wait_with_output()?;
        Ok(CmdOutput {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}
