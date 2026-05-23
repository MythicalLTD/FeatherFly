.PHONY: debug build release run test fmt clippy check audit

debug:
	cargo run -q -- --debug

build:
	cargo build

release:
	cargo build --release

run:
	cargo run

test:
	cargo test --all-targets

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

check: fmt clippy test build

audit:
	cargo audit
