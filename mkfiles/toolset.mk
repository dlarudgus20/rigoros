CONFIG ?= debug

ifeq ($(CONFIG), debug)
else ifeq ($(CONFIG), release)
else
$(error [CONFIG] '$(CONFIG)': unknown configuration.)
endif

TOOLSET_PREFIX ?= x86_64-pc-elf
TOOLSET_GCC ?= $(TOOLSET_PREFIX)-gcc
TOOLSET_AS ?= $(TOOLSET_PREFIX)-as
TOOLSET_OBJCOPY ?= $(TOOLSET_PREFIX)-objcopy
TOOLSET_OBJDUMP ?= $(TOOLSET_PREFIX)-objdump
TOOLSET_NM ?= $(TOOLSET_PREFIX)-nm
TOOLSET_GDB ?= $(TOOLSET_PREFIX)-gdb

TOOLSET_QEMU ?= qemu-system-x86_64
TOOLSET_BOCHS ?= bochs
