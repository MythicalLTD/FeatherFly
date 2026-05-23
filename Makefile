.PHONY: debug build release run test fmt clippy check audit plugin plugin-ship docs setup-hooks

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

audit:
	cargo audit

PLUGIN ?= plugins/hello

plugin: fmt
	$(CARGO_RUN) plugin build $(PLUGIN)

plugin-ship: fmt
	$(CARGO_RUN) plugin ship $(PLUGIN)
