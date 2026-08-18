BIN := __CRATE__
IMAGE ?= __CRATE__:dev

.PHONY: all build test check lint fmt fmt-check release image clean

all: fmt-check lint test check

build:
	cargo build --locked

test:
	cargo test --locked

## strictrs is the primary feedback oracle (JSON report on stdout)
check:
	strictrs check .

lint:
	cargo clippy --all-targets --all-features --locked -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

release:
	cargo build --release --locked

image:
	docker build -t $(IMAGE) .

clean:
	cargo clean
