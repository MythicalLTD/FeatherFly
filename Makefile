.PHONY: debug build release run test fmt clippy check audit plugin plugin-ship docs

CARGO_RUN = cargo run -q --bin featherfly --

debug:
	$(CARGO_RUN) --debug

build:
	cargo build

release:
	cargo build --release

run:
	cargo run --bin featherfly

docs:
	cargo run --bin generate-docs

test:
	cargo test --workspace --all-targets

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

check: fmt clippy test build

audit:
	cargo audit

PLUGIN ?= plugins/hello

plugin:
	$(CARGO_RUN) plugin build $(PLUGIN)

plugin-ship:
	$(CARGO_RUN) plugin ship $(PLUGIN)
