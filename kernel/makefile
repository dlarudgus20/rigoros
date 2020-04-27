TARGET_NAME := kernel
all: build

include ../mkfiles/toolset.mk
include ../mkfiles/conf.mk

ifeq ($(CONFIG), release)
CARGO_FLAG += --release
endif

RUST_OUTPUT_DIR := target/x86_64-unknown-none/$(CONFIG)
RUST_OUTPUT_LIB := $(RUST_OUTPUT_DIR)/libkernel.a
RUST_OUTPUT_DEP := $(RUST_OUTPUT_DIR)/libkernel.d

LIBRARIES := $(RUST_OUTPUT_LIB)
DEPENDENCIES := $(RUST_OUTPUT_DEP)

include ../mkfiles/rules.mk

TARGET_BINARY := $(DIR_BIN)/kernel.sys

build: $(TARGET_BINARY)

$(RUST_OUTPUT_LIB) $(RUST_OUTPUT_DEP): src/lib.rs
	cargo xbuild $(CARGO_FLAG)
	$(TOOLSET_OBJDUMP) $(OBJDUMP_FLAGS) -D $(RUST_OUTPUT_LIB) > $(DIR_OBJ)/libkernel.disasm

$(TARGET_BINARY): $(TARGET_ELF)
	$(TOOLSET_OBJCOPY) -O binary -j .startup -j .text -j .rodata -j .data -j .bss -S -g $< $@
