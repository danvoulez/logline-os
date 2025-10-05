# Convenience Make targets for LogLine testing, fuzzing, benchmarking and docs.

.PHONY: test fuzz bench doc gateway

test:
	cargo test

fuzz:
	cargo fuzz run fuzz_envelope

bench:
	cargo bench

doc:
	cargo doc --open

gateway:
	cargo run -p logline-gateway
