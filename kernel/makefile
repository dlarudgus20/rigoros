TARGET_NAME := kernel
all: build

include ../mkfiles/conf.mk

RUST_OUTPUT_DIR := target/x86_64-unknown-none/$(CARGO_TARGET_DIR)
RUST_OUTPUT_LIB := $(abspath $(RUST_OUTPUT_DIR)/libkernel.a)
RUST_OUTPUT_DEP := $(abspath $(RUST_OUTPUT_DIR)/libkernel.d)

DEPENDENCIES += $(RUST_OUTPUT_DEP)

CARGO_FLAG += -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem
CARGO_DEPS := Cargo.toml x86_64-unknown-none.json .cargo/config.toml rust-toolchain

LIBRARIES := $(RUST_OUTPUT_LIB)

include ../mkfiles/rules.mk

TARGET_BINARY := $(DIR_BIN)/kernel.sys

# rules

.PHONY: $(RUST_OUTPUT_LIB)

build: $(TARGET_BINARY)

cleanimpl:
	cargo clean

$(RUST_OUTPUT_LIB) $(RUST_OUTPUT_DEP): $(CARGO_DEPS)
	cargo build $(CARGO_FLAG)
	$(TOOLSET_OBJDUMP) $(OBJDUMP_FLAGS) -D $(RUST_OUTPUT_LIB) > $(DIR_OBJ)/libkernel.disasm

$(TARGET_BINARY): $(TARGET_ELF)
	$(TOOLSET_OBJCOPY) -O binary -j .startup -j .text -j .rodata -j .data -j .bss -S -g $< $@
