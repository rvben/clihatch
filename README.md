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

### Nix

With Nix’s `nix-command` and `flakes` features enabled, packages are available
for Linux x64/ARM64 and Apple Silicon macOS:

```sh
nix run github:rvben/clihatch -- --help
nix build github:rvben/clihatch
```

The Nix package supplies Git, GitHub CLI, and OpenSSH for repository creation
and secret management. Cargo is optional for formatting generated sources;
`nix develop` provides it.

Both `Cargo.lock` and `flake.lock` are committed. The package runs the Rust test
suite and checks the installed command and Bash, Fish, and Zsh completions.
Intel Macs are not supported by the pinned nixpkgs; use the other installation
methods above.

For NixOS or Home Manager, add the input to your flake:

```nix
inputs.clihatch = {
  url = "github:rvben/clihatch";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

Then add `inputs.clihatch.packages.${pkgs.stdenv.hostPlatform.system}.default`
to `environment.systemPackages` (NixOS) or `home.packages` (Home Manager).
Pass `inputs` through `specialArgs` for `nixosSystem` or `extraSpecialArgs` for
`homeManagerConfiguration`. The overlay `inputs.clihatch.overlays.default`
also provides `pkgs.clihatch` using your package set and Rust toolchain overrides.
Following your own nixpkgs uses its toolchain; it must meet the Rust version
required by `Cargo.toml` and support your platform.

For development:

```sh
nix develop
nix flake check                  # build, Rust tests, installed-command checks
nix fmt -- --check flake.nix nix/*.nix
```

`direnv allow` is optional and requires nix-direnv. The development shell
includes the package's native build dependencies and Rust development tools.
Update Nix inputs deliberately with `nix flake update`, review the lockfile,
and run the checks before committing it.

## Usage

```sh
clihatch new my-tool
cd my-tool && make check      # lint + tests pass out of the box
./target/debug/my-tool 21     # the example command runs
```

Options: `--description`, `--owner` (default `rvben`), `--author` (default: git
config), `--into <dir>`, `--no-git`, `--github`, `--no-pypi`.

Use `--no-pypi` for a Rust-only tool: it omits the PyPI/maturin machinery, so
the crate publishes to crates.io + Homebrew only (and never trips PyPI's
new-project rate limit).

The whole path from nothing to a published release is four steps, and `clihatch`
does three of them:

```sh
clihatch new my-tool --github   # scaffold + git commit + create & push the repo
cd my-tool && make check        # lint + tests, green out of the box
clihatch secrets my-tool        # bootstrap the remaining release secrets
vership release                 # tag + dual-publish
```

`--github` creates the public `owner/name` repo and pushes the initial commit
(via `gh`), so `clihatch secrets` can run immediately. Without it, `clihatch new`
prints the exact `gh repo create` command in its next-steps. Every `new` run
lists the remaining steps through to a release.

### Bootstrap release secrets

Once the repo exists on GitHub, wire up the remaining secrets the release
pipeline needs in one step. crates.io uses OIDC Trusted Publishing, so no
long-lived Cargo token is stored:

```sh
clihatch secrets my-tool            # -> rvben/my-tool
clihatch secrets my-tool --dry-run  # show what would be set, touch nothing
clihatch secrets my-tool --verify   # read-only: which secrets are already set
```

- **`HOMEBREW_TAP_DEPLOY_KEY`** - generates an ed25519 key, registers it as a
  write deploy key on the tap (`--tap`, default `rvben/homebrew-tap`), and
  stores the private key. This is the fiddly part, fully automated.
- **`PYPI_API_TOKEN`** - read from `$PYPI_API_TOKEN` / `$UV_PUBLISH_TOKEN`, the
  `[pypi]` token in `~/.pypirc`, or `--pypi-token-stdin`.

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
- **The release pipeline** - `.github/workflows/{ci,release}.yml` (GitHub-hosted,
  building macOS + Linux). Each target publishes in its own job, so one target's
  failure (e.g. a registry rate limit) doesn't block the rest - re-run just the
  failed one with `gh run rerun --failed`. Includes `pyproject.toml` (maturin)
  for the PyPI wheel unless `--no-pypi`, plus `Makefile`, `prek.toml`,
  `README.md`, `LICENSE`, `.gitignore`.
- **Nix packaging** - `flake.nix`, `nix/package.nix`, a development shell, shell
  completions, native Linux/macOS CI, and committed `Cargo.lock` + `flake.lock`.
  Lockfiles are rendered from tested templates, so scaffolding needs neither
  network access nor a Nix installation.
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

## Releasing

Vership owns versioning, changelog generation, release commits, and tags. See
[the release runbook](docs/releases.md) for the verified workflow and recovery policy.

## Maintaining the templates

The checked-in lock templates make a newly generated repository reproducible.
Run `scripts/update-template-locks.sh` when updating template dependencies or
nixpkgs. It resolves dependencies in a temporary generated project, then refreshes
`templates/Cargo.lock.tmpl`, `flake.lock`, and `templates/flake.lock.tmpl`.
Review the resulting changes and run `make check`,
`cargo test --locked --test compiles -- --ignored`, and `nix flake check`.
The explicit Cargo compile check fails on registry errors; it never silently
passes. Nix additionally builds and tests an actual generated project without
network access in the build sandbox or import-from-derivation during evaluation.
