//! The scaffolding engine: walk the embedded templates, substitute
//! placeholders, write the tree, and (optionally) make the first git commit.

use std::fs;
use std::path::Path;
use std::process::Command;

use include_dir::{Dir, include_dir};

use crate::error::ClihatchError;

/// The template tree, embedded at compile time.
static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Substitution values for one scaffold.
#[derive(Debug, Clone)]
pub struct Vars {
    pub name: String,
    pub name_snake: String,
    pub name_pascal: String,
    pub description: String,
    pub owner: String,
    pub author: String,
    pub year: String,
}

impl Vars {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        owner: impl Into<String>,
        author: impl Into<String>,
        year: impl Into<String>,
    ) -> Self {
        let name = name.into();
        let name_snake = name.replace('-', "_");
        let name_pascal = name
            .split(['-', '_'])
            .filter(|s| !s.is_empty())
            .map(capitalize)
            .collect();
        Vars {
            name,
            name_snake,
            name_pascal,
            description: description.into(),
            owner: owner.into(),
            author: author.into(),
            year: year.into(),
        }
    }

    /// Replace every `{{placeholder}}` in `s`. The longer tokens
    /// (`{{name_snake}}`, `{{name_pascal}}`) are distinct strings from
    /// `{{name}}`, so replacement order does not matter.
    pub fn apply(&self, s: &str) -> String {
        s.replace("{{name_snake}}", &self.name_snake)
            .replace("{{name_pascal}}", &self.name_pascal)
            .replace("{{name}}", &self.name)
            .replace("{{description}}", &self.description)
            .replace("{{owner}}", &self.owner)
            .replace("{{author}}", &self.author)
            .replace("{{year}}", &self.year)
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// What was created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub name: String,
    /// The GitHub owner the next-steps target (`owner/name`).
    pub owner: String,
    pub dir: String,
    pub files: Vec<String>,
    pub committed: bool,
    /// The `owner/name` of the GitHub repo created with `--github`, if any.
    pub repo: Option<String>,
}

/// Scaffold a new crate into `into/<name>`. Refuses if the directory exists.
pub fn scaffold(into: &Path, vars: &Vars, git: bool) -> Result<Outcome, ClihatchError> {
    let dir = into.join(&vars.name);
    if dir.exists() {
        return Err(ClihatchError::Exists {
            path: dir.display().to_string(),
        });
    }

    let mut files = Vec::new();
    write_dir(&TEMPLATES, &dir, vars, &mut files)?;
    files.sort();

    // Format the generated sources so they are rustfmt-clean regardless of how
    // the templates are laid out. Best-effort: rustfmt ships with the default
    // toolchain, and a missing one only means the templates' own formatting
    // stands.
    let _ = Command::new("cargo").current_dir(&dir).arg("fmt").output();

    let committed = if git {
        git_init(&dir, &vars.name, &vars.author, &files)?;
        true
    } else {
        false
    };

    Ok(Outcome {
        name: vars.name.clone(),
        owner: vars.owner.clone(),
        dir: dir.display().to_string(),
        files,
        committed,
        repo: None,
    })
}

fn write_dir(
    dir: &Dir,
    dest_root: &Path,
    vars: &Vars,
    files: &mut Vec<String>,
) -> Result<(), ClihatchError> {
    for file in dir.files() {
        let rel = file.path().to_string_lossy();
        let out_rel = rel.strip_suffix(".tmpl").unwrap_or(&rel).to_string();
        let target = dest_root.join(&out_rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        let content = file.contents_utf8().ok_or_else(|| ClihatchError::Io {
            message: format!("template {rel} is not valid UTF-8"),
        })?;
        fs::write(&target, vars.apply(content)).map_err(io)?;
        files.push(out_rel);
    }
    for sub in dir.dirs() {
        write_dir(sub, dest_root, vars, files)?;
    }
    Ok(())
}

/// `git init` + add exactly the generated files + an initial commit. Never
/// `git add -A`: only the files we created are staged. The branch is forced to
/// `main` so it matches the `on: push: main` triggers in the generated CI,
/// regardless of the user's `init.defaultBranch`.
///
/// The commit identity is set explicitly from `author`, so scaffolding succeeds
/// even where `git config user.name/user.email` is unset (e.g. fresh CI).
fn git_init(dir: &Path, name: &str, author: &str, files: &[String]) -> Result<(), ClihatchError> {
    run_git(dir, &["init", "-q"])?;
    let mut args: Vec<&str> = vec!["add", "--"];
    args.extend(files.iter().map(String::as_str));
    run_git(dir, &args)?;

    let (commit_name, commit_email) = author_identity(author);
    let message = format!("chore: scaffold {name} with clihatch");
    run_git(
        dir,
        &[
            "-c",
            &format!("user.name={commit_name}"),
            "-c",
            &format!("user.email={commit_email}"),
            "commit",
            "-q",
            "-m",
            &message,
        ],
    )?;
    run_git(dir, &["branch", "-M", "main"])?;
    Ok(())
}

/// Split an `author` string of the form `Name <email>` into `(name, email)`,
/// falling back to safe placeholders so the commit always has an identity.
fn author_identity(author: &str) -> (String, String) {
    let (name, email) = match (author.find('<'), author.rfind('>')) {
        (Some(open), Some(close)) if close > open => {
            (author[..open].trim(), author[open + 1..close].trim())
        }
        _ => (author.trim(), ""),
    };
    let name = if name.is_empty() { "clihatch" } else { name };
    let email = if email.is_empty() {
        "clihatch@users.noreply.github.com"
    } else {
        email
    };
    (name.to_string(), email.to_string())
}

fn run_git(dir: &Path, args: &[&str]) -> Result<(), ClihatchError> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| ClihatchError::Git {
            message: format!("could not run git: {e}"),
        })?;
    if !output.status.success() {
        return Err(ClihatchError::Git {
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(())
}

fn io(e: std::io::Error) -> ClihatchError {
    ClihatchError::Io {
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Vars, author_identity};

    #[test]
    fn splits_author_into_name_and_email() {
        assert_eq!(
            author_identity("Ruben Jongejan <ruben@example.com>"),
            (
                "Ruben Jongejan".to_string(),
                "ruben@example.com".to_string()
            )
        );
    }

    #[test]
    fn author_identity_falls_back_when_malformed() {
        // No angle brackets -> the whole string is the name, placeholder email.
        let (name, email) = author_identity("Just A Name");
        assert_eq!(name, "Just A Name");
        assert!(email.contains('@'));
        // Empty -> safe placeholders, never empty (git rejects empty idents).
        let (name, email) = author_identity("");
        assert!(!name.is_empty() && email.contains('@'));
    }

    fn vars(name: &str) -> Vars {
        Vars::new(name, "desc", "rvben", "A <a@b.c>", "2026")
    }

    #[test]
    fn derives_snake_and_pascal_case() {
        let v = vars("my-cool-tool");
        assert_eq!(v.name_snake, "my_cool_tool");
        assert_eq!(v.name_pascal, "MyCoolTool");
    }

    #[test]
    fn substitutes_all_placeholders() {
        let v = vars("foo-bar");
        let out = v.apply("{{name}} {{name_snake}} {{name_pascal}} {{owner}} {{year}}");
        assert_eq!(out, "foo-bar foo_bar FooBar rvben 2026");
    }

    #[test]
    fn name_token_does_not_clobber_longer_tokens() {
        // `{{name}}` replacement must not corrupt `{{name_snake}}`.
        let v = vars("x");
        assert_eq!(v.apply("{{name_snake}}"), "x");
        assert_eq!(v.apply("{{name_pascal}}"), "X");
    }
}
