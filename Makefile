.PHONY: fmt fmt-check clippy test build run preflight

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

clippy:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test

build:
	cargo build

run:
	cargo run

# The one command CI runs — keep it green as coverage grows (#46).
preflight: fmt-check clippy test
