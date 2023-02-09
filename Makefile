.PHONY: run test build

run_%:
	cargo run --example $*

test:
	cargo test --lib

build:
	cargo fmt && \
	cargo clippy && \
	cargo build

publish/dryrun:
	cargo publish --dry-run
