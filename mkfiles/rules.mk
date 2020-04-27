ifndef TARGET_NAME
$(error target name must be specified)
endif

LIBRARIES +=

C_SOURCES += $(wildcard $(DIR_SRC)/*.c)
C_OBJECTS := $(patsubst $(DIR_SRC)/%.c, $(DIR_OBJ)/%.c.o, $(C_SOURCES))

AS_SOURCES += $(wildcard $(DIR_SRC)/*.S)
AS_OBJECTS := $(patsubst $(DIR_SRC)/%.S, $(DIR_OBJ)/%.S.o, $(AS_SOURCES))

DEPENDENCIES += $(patsubst $(DIR_SRC)/%.c, $(DIR_DEP)/%.c.d, $(C_SOURCES)) \
	$(patsubst $(DIR_SRC)/%.S, $(DIR_DEP)/%.S.d, $(AS_SOURCES))

LD_SCRIPT := linker.ld

TARGET_ELF := $(DIR_BIN)/$(TARGET_NAME).elf

PHONY_TARGETS += all build rebuild mostlyclean clean cleanimpl
.PHONY: $(PHONY_TARGETS)

rebuild:
	make clean
	make build

mostlyclean:
	-rm -rf $(DIR_OBJ)
	-rm -rf $(DIR_DEP)
	-make cleanimpl

clean: mostlyclean
	-rm -rf $(DIR_BIN)

distclean: clean clean_dirs

$(TARGET_ELF): $(LD_SCRIPT) $(C_OBJECTS) $(AS_OBJECTS) $(LIBRARIES) | $(DIRS)
	$(TOOLSET_GCC) $(LDFLAGS) -T $(LD_SCRIPT) -o $@ $(C_OBJECTS) $(AS_OBJECTS) $(LIBRARIES) \
		-Xlinker -Map=$(DIR_OBJ)/$(TARGET_NAME).map
	$(TOOLSET_NM) $(NM_FLAGS) $@ > $(DIR_OBJ)/$(TARGET_NAME).nm
	$(TOOLSET_OBJDUMP) $(OBJDUMP_FLAGS) -D $@ > $(DIR_OBJ)/$(TARGET_NAME).disasm

	$(TOOLSET_NM) --numeric-sort $@ \
		| perl -p -e 's/([0-9a-fA-F]*) ([0-9a-fA-F]* .|.) ([^\s]*)(^$$|.*)/\1 \3/g' \
		> $(DIR_OBJ)/$(TARGET_NAME).sym

$(DIR_OBJ)/%.c.o: $(DIR_SRC)/%.c | $(DIRS)
	$(TOOLSET_GCC) $(CFLAGS) -c $< -o $@
	$(TOOLSET_OBJDUMP) $(OBJDUMP_FLAGS) -D $@ > $(DIR_OBJ)/$*.c.dump

$(DIR_DEP)/%.c.d: $(DIR_SRC)/%.c | $(DIRS)
	$(TOOLSET_GCC) $(CFLAGS) $< -MM -MT $(DIR_OBJ)/$*.c.o \
		| sed 's@\($(DIR_OBJ)/$*.c.o\)[ :]*@\1 $@ : @g' > $@

$(DIR_OBJ)/%.S.o: $(DIR_SRC)/%.S | $(DIRS)
	$(TOOLSET_GCC) $(ASFLAGS) -c $< -o $@ \
		-Xassembler -a=$(DIR_OBJ)/$*.S.lst
	$(TOOLSET_OBJDUMP) $(OBJDUMP_FLAGS) -D $@ > $(DIR_OBJ)/$*.S.dump

$(DIR_DEP)/%.S.d: $(DIR_SRC)/%.S | $(DIRS)
	$(TOOLSET_GCC) $(ASFLAGS) $< -MM -MT $(DIR_OBJ)/$*.S.o \
		| sed 's@\($(DIR_OBJ)/$*.S.o\)[ :]*@\1 $@ : @g' > $@

ifeq ($(filter $(subst build, , $(PHONY_TARGETS)), $(MAKECMDGOALS)), )
include $(DEPENDENCIES)
endif
