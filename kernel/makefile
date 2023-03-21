TARGET_NAME := kernel
all: build

include ../mkfiles/conf.mk

RUST_OUTPUT_DIR := target/x86_64-unknown-none/$(CARGO_TARGET_DIR)
RUST_OUTPUT_LIB := $(abspath $(RUST_OUTPUT_DIR)/libkernel.a)
RUST_OUTPUT_DEP := $(abspath $(RUST_OUTPUT_DIR)/libkernel.d)

DEPENDENCIES += $(RUST_OUTPUT_DEP)

CUSTOM_TARGET := x86_64-unknown-none.json
CARGO_FLAG += --target $(CUSTOM_TARGET) -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem
CARGO_DEPS := Cargo.toml $(CUSTOM_TARGET) rust-toolchain

CODE_SECTIONS := .startup .text

LIBRARIES := $(RUST_OUTPUT_LIB)

PHONY_TARGETS += test

include ../mkfiles/rules.mk

TARGET_BINARY := $(DIR_BIN)/kernel.sys

# rules

build: $(TARGET_BINARY)

test:
	cargo test

cleanimpl:
	cargo clean

$(RUST_OUTPUT_LIB) $(RUST_OUTPUT_DEP): $(CARGO_DEPS)
	cargo build $(CARGO_FLAG)

$(TARGET_BINARY): $(TARGET_ELF)
	$(TOOLSET_OBJCOPY) -O binary -j .startup -j .text -j .rodata -j .data -j .bss -S -g $< $@
