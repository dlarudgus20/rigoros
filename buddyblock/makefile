all: build

include ../mkfiles/conf.mk

RUST_OUTPUT_DIR := target/$(CARGO_TARGET_DIR)
RUST_OUTPUT_LIB := $(abspath $(RUST_OUTPUT_DIR)/libbuddyblock.rlib)
RUST_OUTPUT_DEP := $(abspath $(RUST_OUTPUT_DIR)/libbuddyblock.d)

DEPENDENCIES += $(RUST_OUTPUT_DEP)

CARGO_DEPS := Cargo.toml

include ../mkfiles/rules.mk

# rules

build: $(RUST_OUTPUT_LIB)

test:
	cargo test

cleanimpl:
	cargo clean

$(RUST_OUTPUT_LIB) $(RUST_OUTPUT_DEP): $(CARGO_DEPS)
	cargo build $(CARGO_FLAG)
