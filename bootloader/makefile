TARGET_NAME := boot
all: build

STAGE_SECTORS = $(shell echo $$(((`stat --printf="%s" ../kernel/bin/$(CONFIG)/kernel.sys` + 511) / 512 )))

ASFLAGS += -DSTAGE_SECTORS=$(STAGE_SECTORS)
OBJDUMP_FLAGS += -Mi8086

include ../mkfiles/toolset.mk
include ../mkfiles/conf.mk
include ../mkfiles/rules.mk

BOOT := $(DIR_BIN)/boot.bin

build: $(BOOT) $(STAGE)
	@echo Bootloader will load $(STAGE_SECTORS) sector\(s\).

$(BOOT): $(TARGET_ELF)
	$(TOOLSET_OBJCOPY) -O binary -j .boot -S -g $< $@
