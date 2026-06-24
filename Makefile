.PHONY: check test fmt web-build dev build install-cross build-linux-x86_64 build-linux-aarch64 build-linux-all

check:
	cargo check

test:
	cargo test

fmt:
	cargo fmt

web-build:
	npm --prefix web run build

dev:
	npm run dev

build:
	npm run build

install-cross:
	cargo install cross --locked

build-linux-x86_64:
	npm run build

build-linux-aarch64:
	cross build --release --target aarch64-unknown-linux-gnu

build-linux-all: build-linux-x86_64 build-linux-aarch64
