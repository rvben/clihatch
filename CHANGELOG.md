# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).






## [0.1.5](https://github.com/rvben/clihatch/compare/v0.1.4...v0.1.5) - 2026-06-22

### Added

- isolate release-pipeline jobs, add --no-pypi and secrets --verify ([5619b92](https://github.com/rvben/clihatch/commit/5619b923ea80ad65948b7ebaf85e2c2e7feee8d5))

### Fixed

- **secrets**: default --tap to <owner>/homebrew-tap for owner consistency ([2052238](https://github.com/rvben/clihatch/commit/2052238a7a1fc854ee11cdfaa5eee94e57f2f631))

## [0.1.4](https://github.com/rvben/clihatch/compare/v0.1.3...v0.1.4) - 2026-06-21

### Added

- **new**: add --github to create+push the repo, and full-lifecycle next-steps ([976eb23](https://github.com/rvben/clihatch/commit/976eb23731a627f18efd0f29b2fdd3c6dd1a87a0))

### Fixed

- **new**: carry --owner into next-steps and force the initial branch to main ([8a68c62](https://github.com/rvben/clihatch/commit/8a68c625e59c0436ae6ae49e51ea1b3339515846))

## [0.1.3](https://github.com/rvben/clihatch/compare/v0.1.2...v0.1.3) - 2026-06-21

### Added

- **secrets**: read PyPI token from ~/.pypirc ([0caedd9](https://github.com/rvben/clihatch/commit/0caedd99bfe8a7b048671650990a59d3495c48aa))

## [0.1.2](https://github.com/rvben/clihatch/compare/v0.1.1...v0.1.2) - 2026-06-21

### Added

- **secrets**: add gh preflight, idempotent key rotation, and a CommandRunner test seam ([13c1fc2](https://github.com/rvben/clihatch/commit/13c1fc2a36ec33906b20cd2543984eda214cda07))

## [0.1.1](https://github.com/rvben/clihatch/compare/v0.1.0...v0.1.1) - 2026-06-21

### Added

- add secrets subcommand to bootstrap release secrets ([f62d3bd](https://github.com/rvben/clihatch/commit/f62d3bd23a1648282621c791d145b68b7cb70bd1))

## [0.1.0] - 2026-06-20

### Added

- clihatch - scaffold a clispec-compliant Rust CLI ([a6aabe2](https://github.com/rvben/clihatch/commit/a6aabe20e0d7d81c1fa0074d3d87fdf6bc7c1fbe))
