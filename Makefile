.PHONY: build release test lint fmt check conformance clean install release-patch release-minor release-major update-deps

build:
	cargo build

release:
	cargo build --release

test:
	cargo nextest run

lint:
	cargo fmt -- --check
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

check: lint test

# Score the binary against The CLI Spec (clispec.dev). Requires `clispec`
# (cargo install clispec). The schema's conformance to clispec v0.2 is also
# verified hermetically by `make test`.
conformance: release
	clispec score ./target/release/clihatch

clean:
	cargo clean

install: release
	mkdir -p ~/.local/bin
	cp target/release/clihatch ~/.local/bin/clihatch

update-deps:
	upd --apply --max-bump minor --lang rust,actions

release-patch:
	vership bump patch

release-minor:
	vership bump minor

release-major:
	vership bump major
