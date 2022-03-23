TARGET_NAME := boot
all: build

include ../mkfiles/conf.mk
include ../mkfiles/color.mk

STAGE_BINARY := ../kernel/bin/$(CONFIG)/kernel.sys
STAGE_SIZE := $(shell echo `stat --printf="%s" $(STAGE_BINARY)`)
STAGE_SECTORS := $(shell echo $$(( ($(STAGE_SIZE) + 511) / 512 )))

ASFLAGS += -DSTAGE_SECTORS=$(STAGE_SECTORS)
OBJDUMP_FLAGS += -Mi8086

include ../mkfiles/rules.mk

BOOT := $(DIR_BIN)/boot.bin

build: $(BOOT) $(STAGE)
	@echo "${FG_LGREEN}Stage size is $(STAGE_SIZE) byte(s).${NO_COLOR}"
	@echo "${FG_LGREEN}Bootloader will load $(STAGE_SECTORS) sector(s).${NO_COLOR}"

$(BOOT): $(TARGET_ELF)
	$(TOOLSET_OBJCOPY) -O binary -j .boot -S -g $< $@

obj/$(CONFIG)/boot.S.o: $(STAGE_BINARY)
