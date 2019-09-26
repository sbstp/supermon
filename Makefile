.PHONY: release
release:
	cargo build --release
	strip target/release/supermon
	cp target/release/supermon .

.PHONY: tests
tests:
	docker build -f tests/zombies/Dockerfile -t supermon-test-zombies .
