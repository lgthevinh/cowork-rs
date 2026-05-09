.PHONY: check test fmt build install-cross build-linux-x86_64 build-linux-aarch64 build-linux-all

check:
	cargo check

test:
	cargo test

fmt:
	cargo fmt

build:
	cargo build --release

install-cross:
	cargo install cross --locked

build-linux-x86_64:
	cargo build --release --target x86_64-unknown-linux-gnu

build-linux-aarch64:
	cross build --release --target aarch64-unknown-linux-gnu

build-linux-all: build-linux-x86_64 build-linux-aarch64
