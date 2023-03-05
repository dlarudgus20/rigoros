CONFIG ?= debug

ifeq ($(CONFIG), debug)
else ifeq ($(CONFIG), release)
else
$(error [CONFIG] '$(CONFIG)': unknown configuration.)
endif

TOOLSET_PREFIX ?= x86_64-elf
TOOLSET_GCC ?= $(TOOLSET_PREFIX)-gcc
TOOLSET_AS ?= $(TOOLSET_PREFIX)-as
TOOLSET_OBJCOPY ?= $(TOOLSET_PREFIX)-objcopy
TOOLSET_OBJDUMP ?= $(TOOLSET_PREFIX)-objdump
TOOLSET_NM ?= $(TOOLSET_PREFIX)-nm
TOOLSET_GDB ?= $(TOOLSET_PREFIX)-gdb

TOOLSET_QEMU ?= qemu-system-x86_64
TOOLSET_BOCHS ?= bochs

ifeq ($(CONFIG), debug)
CFLAGS += -DDEBUG -ggdb
ASFLAGS += -DDEBUG -g3
CARGO_TARGET_DIR := debug
CARGO_FLAG += --profile dev
else ifeq ($(CONFIG), release)
CFLAGS += -DNDEBUG -O3 -flto
ASFLAGS += -DNDEBUG
CARGO_TARGET_DIR := release
CARGO_FLAG += --profile release
endif

CFLAGS += -ffreestanding -mcmodel=large -mno-red-zone -mno-mmx -mno-sse -mno-sse2 -fpack-struct \
	-std=c99 -Wall -Wextra -Werror -Wno-unused-parameter -Wno-error=unused-variable -Wno-error=unused-function
ASFLAGS +=
LDFLAGS += -ffreestanding -nostdlib -Xlinker --gc-sections
OBJDUMP_FLAGS += -C
NM_FLAGS += -C --line-numbers --print-size --print-armap --numeric-sort

DIR_SRC := src
DIR_BIN := bin/$(CONFIG)
DIR_OBJ := obj/$(CONFIG)
DIR_DEP := dep/$(CONFIG)
DIRS := $(DIR_BIN) $(DIR_OBJ)

PHONY_TARGETS += clean_dirs

$(DIRS):
	mkdir -p $(DIR_BIN)
	mkdir -p $(DIR_OBJ)
	mkdir -p $(DIR_DEP)

clean_dirs:
	-rm -rf $(DIR_BIN)
	-rm -rf $(DIR_OBJ)
	-rm -rf $(DIR_DEP)
	-rmdir bin
	-rmdir obj
	-rmdir dep
