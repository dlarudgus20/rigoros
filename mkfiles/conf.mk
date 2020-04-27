CONFIG ?= debug

CFLAGS += -ffreestanding -mcmodel=large -mno-red-zone -mno-mmx -mno-sse -mno-sse2 -fpack-struct \
	-std=c99 -Wall -Wextra -Werror -Wno-unused-parameter -Wno-error=unused-variable -Wno-error=unused-function
ASFLAGS +=
LDFLAGS += -ffreestanding -nostdlib -Xlinker --gc-sections
OBJDUMP_FLAGS +=
# NM_FLAGS += --line-numbers --print-size --print-armap --numeric-sort
NM_FLAGS += --print-size --print-armap --numeric-sort

ifeq ($(CONFIG), debug)
CFLAGS += -DDEBUG -ggdb
ASFLAGS += -DDEBUG -ggdb
else ifeq ($(CONFIG), release)
CFLAGS += -DNDEBUG -O3 -flto
ASFLAGS += -DNDEBUG
else
$(error [CONFIG] '$(CONFIG)': unknown configuration.)
endif

DIR_SRC := src
DIR_BIN := bin/$(CONFIG)
DIR_OBJ := obj/$(CONFIG)
DIR_DEP := dep/$(CONFIG)
DIRS := $(DIR_BIN) $(DIR_OBJ)

$(DIRS):
	mkdir -p $(DIR_BIN)
	mkdir -p $(DIR_OBJ)
	mkdir -p $(DIR_DEP)

clean_dirs:
	-rm -rf $(DIR_BIN)
	-rm -rf $(DIR_OBJ)
	-rm -rf $(DIR_DEP)
