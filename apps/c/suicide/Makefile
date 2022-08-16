SRC_LIBSEL4 ?= $(ROOTDIR)/cantrip/kernel/libsel4
OUT_CANTRIP    ?= $(OUT)/cantrip/riscv32-unknown-elf/release
OUT_TMP     ?= $(OUT)/tmp/suicide

INCLUDES += -I$(SRC_LIBSEL4)/arch_include/riscv
INCLUDES += -I$(SRC_LIBSEL4)/include
INCLUDES += -I$(SRC_LIBSEL4)/mode_include/32
INCLUDES += -I$(SRC_LIBSEL4)/sel4_arch_include/riscv32/
INCLUDES += -I$(OUT_CANTRIP)/kernel/gen_config
INCLUDES += -I$(OUT_CANTRIP)/libsel4/autoconf
INCLUDES += -I$(OUT_CANTRIP)/libsel4/gen_config/
INCLUDES += -I$(OUT_CANTRIP)/libsel4/include
INCLUDES += -I$(OUT_CANTRIP)/libsel4/sel4_arch_include/riscv32

OPT:=-O0
DBG:=-g

$(OUT_TMP)/suicide.elf: suicide.c
	mkdir -p $(OUT_TMP)
	riscv32-unknown-elf-gcc $(DBG) $(OPT) $(INCLUDES) -march=rv32imac -mabi=ilp32 -std=gnu11 -c suicide.c -o $(OUT_TMP)/suicide.o
	riscv32-unknown-elf-gcc $(DBG) -static -nostdlib $(OUT_TMP)/suicide.o -o $(OUT_TMP)/suicide.elf
