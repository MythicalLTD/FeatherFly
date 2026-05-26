.PHONY: debug build release run test fmt clippy check audit docs setup-hooks ci

CARGO_RUN = cargo run -q --bin featherfly --

fmt:
	cargo fmt --all

setup-hooks:
	chmod +x .githooks/pre-commit
	git config core.hooksPath .githooks
	@echo "git hooks installed (.githooks/pre-commit runs cargo fmt before each commit)"

debug: fmt
	$(CARGO_RUN) --debug

build: fmt
	cargo build

release: fmt
	cargo build --release

run: fmt
	cargo run --bin featherfly

docs: fmt
	cargo run --bin generate-docs

test: fmt
	cargo test --workspace --all-targets

clippy: fmt
	cargo clippy --all-targets --all-features -- -D warnings

check: fmt clippy test build

ci: fmt
	cargo fmt --all -- --check
	cargo run --bin generate-docs
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test --workspace --all-targets
	cargo build

audit:
	cargo audit
