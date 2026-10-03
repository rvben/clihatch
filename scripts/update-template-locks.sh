#!/bin/sh
# Maintainer operation; normal scaffolding never resolves dependencies.
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
cargo run --locked --manifest-path "$repo/Cargo.toml" -- new clihatch-template-lock \
  --no-git --no-pypi --into "$work" --author Template --owner example
cargo update --manifest-path "$work/clihatch-template-lock/Cargo.toml"
sed 's/name = "clihatch-template-lock"/name = "{{name}}"/' \
  "$work/clihatch-template-lock/Cargo.lock" > "$repo/templates/Cargo.lock.tmpl"
nix flake update --flake "$repo"
cp "$repo/flake.lock" "$repo/templates/flake.lock.tmpl"
