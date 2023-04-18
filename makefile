all: build

include mkfiles/conf.mk

TOOLS_EDIMG := tools/edimg/edimg

BOOTLOADER_BOOT := bootloader/$(DIR_BIN)/boot.bin
KERNEL_ELF := kernel/$(DIR_BIN)/kernel.elf
KERNEL_BINARY := kernel/$(DIR_BIN)/kernel.sys

RAW_IMAGE := img/raw_floppy.img
TARGET_IMAGE := $(DIR_BIN)/floppy.img

QEMU_DRIVES := -drive "file=$(TARGET_IMAGE)",index=0,if=floppy,format=raw,readonly=on
QEMU_FLAGS := -L . -m 64 $(QEMU_DRIVES) -boot a -rtc base=localtime -M pc -serial stdio
BOCHSRC := bochsrc.bxrc

SUBDIRS := buddyblock slab_alloc kernel bootloader

.PHONY: all build re rebuild run rerun dbg debug gdb bochs test mostlyclean clean distclean

build:
	make -C tools
	for dir in $(SUBDIRS); do \
		make build -C $$dir || exit 1; \
	done
	make $(TARGET_IMAGE)

re: rebuild
rebuild: clean build

run: build
	$(TOOLSET_QEMU) $(QEMU_FLAGS)

rerun: clean run

dbg: debug
debug: build
	$(TOOLSET_QEMU) $(QEMU_FLAGS) -S -gdb tcp:127.0.0.1:1234 \
		-fw_cfg name=opt/org.starrios.debug,string=1

gdb:
	$(TOOLSET_GDB) $(KERNEL_ELF) "-ex=target remote :1234"

bochs: build
	CONFIG=$(CONFIG) $(TOOLSET_BOCHS) -qf $(BOCHSRC)

test:
	for dir in $(SUBDIRS); do \
		make test -C $$dir || exit 1; \
	done

mostlyclean:
	for dir in $(SUBDIRS); do \
		make mostlyclean -C $$dir || exit 1; \
	done

clean:
	for dir in $(SUBDIRS); do \
		make clean -C $$dir || exit 1; \
	done
	-rm -rf $(DIR_BIN)/*

distclean:
	for dir in $(SUBDIRS); do \
		make distclean -C $$dir || exit 1; \
	done
	make distclean -C tools
	-rm -rf $(DIR_BIN)

$(TARGET_IMAGE): $(BOOTLOADER_BOOT) $(BOOTLOADER_STAGE) $(KERNEL_BINARY)
	mkdir -p $(DIR_BIN)
	$(TOOLS_EDIMG) imgin:$(RAW_IMAGE) \
		wbinimg src:$(BOOTLOADER_BOOT) len:512 from:0 to:0 \
		copy from:$(KERNEL_BINARY) to:@: \
		imgout:$(TARGET_IMAGE)
