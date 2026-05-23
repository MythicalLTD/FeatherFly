.PHONY: debug build release run

debug:
	cargo run -q -- --debug

build:
	cargo build

release:
	cargo build --release

run:
	cargo run
