.PHONY: release
release:
	cargo build --release
	strip target/release/supermon
	cp target/release/supermon .
