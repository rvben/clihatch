# clihatch

[![CI](https://github.com/rvben/clihatch/actions/workflows/ci.yml/badge.svg)](https://github.com/rvben/clihatch/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/clihatch.svg)](https://crates.io/crates/clihatch)
[![clispec](https://img.shields.io/badge/clispec-v0.2-blue)](https://clispec.dev)

Scaffold a complete, [clispec](https://clispec.dev)-compliant, agent-facing
Rust CLI in seconds - source skeleton, schema + conformance test, and the
GitHub-hosted dual-publish release pipeline. No more copying your last tool and
sed-ing the name.

## Install

```sh
cargo install clihatch
```

## Usage

```sh
clihatch new my-tool
cd my-tool && make check      # lint + tests pass out of the box
./target/debug/my-tool 21     # the example command runs
```

Options: `--description`, `--owner` (default `rvben`), `--author` (default: git
config), `--into <dir>`, `--no-git`.

### Bootstrap release secrets

Once the repo exists on GitHub, wire up the three secrets the release pipeline
needs in one step:

```sh
clihatch secrets my-tool            # -> rvben/my-tool
clihatch secrets my-tool --dry-run  # show what would be set, touch nothing
```

- **`HOMEBREW_TAP_DEPLOY_KEY`** - generates an ed25519 key, registers it as a
  write deploy key on the tap (`--tap`, default `rvben/homebrew-tap`), and
  stores the private key. This is the fiddly part, fully automated.
- **`CARGO_REGISTRY_TOKEN`** - read from `$CARGO_REGISTRY_TOKEN` or
  `~/.cargo/credentials.toml`.
- **`PYPI_API_TOKEN`** - read from `$PYPI_API_TOKEN` / `$UV_PUBLISH_TOKEN`, or
  `--pypi-token-stdin`.

It preflights `gh` auth and repo access, so it fails fast (before generating a
key) if you are not logged in. Re-running is idempotent: it rotates the deploy
key (dropping the prior key with the same title) so the key and the stored
secret stay in sync. Missing token sources are skipped with a hint, never
invented.

## What you get

A ready-to-`cargo build`, ready-to-release crate:

- **`src/`** - a minimal but complete clispec CLI: a default command, `schema`,
  `completions`, the structured-error envelope, exit-code contract, and
  TTY-aware `-o auto|json|text`. Replace the example `run` logic with yours.
- **`schemas/clispec-v0.2.json` + `tests/conformance.rs`** - your `schema`
  output is validated against the spec by the test suite.
- **`tests/cli.rs`** - end-to-end tests of the binary.
- **The dual-publish pipeline** - `.github/workflows/{ci,release}.yml`
  (GitHub-hosted, building macOS + Linux for crates.io + PyPI + Homebrew),
  `pyproject.toml` (maturin), `Makefile`, `prek.toml`, `README.md`, `LICENSE`,
  `.gitignore`.
- A `git init` + initial commit (skip with `--no-git`). Generated sources are
  `cargo fmt`-clean.

clihatch is itself built with its own output's conventions - it eats its own
dog food, and its test suite scaffolds a crate and compiles it to prove the
templates stay valid.

## Exit codes

| code | meaning |
| --- | --- |
| `0` | success |
| `2` | IO, git, or backend (`gh`/`ssh-keygen`) failure |
| `3` | usage error, or the target directory already exists |

## For agents (clispec)

```sh
clihatch schema
```

Structured output on stdout, structured error envelopes on stderr, a `schema`
subcommand validated against `clispec.dev/schema/v0.2.json`. `new` and `secrets`
are the `mutating: true` commands; `new` never overwrites, and `secrets`
supports `--dry-run`.

## License

MIT
