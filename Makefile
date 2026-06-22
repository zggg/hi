.PHONY: build test lint check-consistency

build:
	cargo build

test:
	cargo test -p architecture-tests

lint:
	cargo clippy --workspace -- -D warnings

check-consistency:
	./scripts/check-consistency.sh
