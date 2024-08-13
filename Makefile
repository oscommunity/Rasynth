.PHONY: all
all: rust

.PNONY: rust
rust:
	cd rasynth && cargo build

.PHONY: run
run: rust
	cd rasynth && ./target/debug/rasynth --box

.PHONY: run-d
run-d: rust
	cd rasynth && ./target/debug/rasynth --display
