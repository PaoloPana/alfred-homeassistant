BIN_FILE := alfred-homeassistant

build:
	cargo build
build-release:
	cargo build --release
aarch64:
	cross build --release --target aarch64-unknown-linux-gnu

install: clean-bin build
	mkdir bin
	cp target/debug/${BIN_FILE} bin/
install-aarch64: clean-bin aarch64
	mkdir bin
	cp target/aarch64-unknown-linux-gnu/release/${BIN_FILE} bin/

clean: clean-target clean-bin
clean-target:
	rm -rf target
clean-bin:
	rm -rf bin