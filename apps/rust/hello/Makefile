OUT_CANTRIP    ?= $(OUT)/cantrip/riscv32-unknown-elf/release
OUT_HELLO   ?= $(OUT_CANTRIP)/apps/hello
CARGO_OPTS  ?= --release

# hello app staticlib (location set w/ --out-dir below)
LIB_HELLO   := ${OUT_HELLO}/libhello.a
# To satisfy Rust core deps with debug build
LIB_LIBC    := ${OUT_CANTRIP}/musllibc/build-temp/stage/lib/libc.a

LD_FLAGS := -march=rv32imac -mabi=ilp32 -static -nostdlib -ftls-model=local-exec
#DBG:=-g

# NB: let cargo handle incremental build steps
${OUT_HELLO}/hello.elf: hello.rs | tmp_check
	SEL4_OUT_DIR=${OUT_CANTRIP}/kernel kcargo build ${CARGO_OPTS} --target-dir ${OUT_HELLO} --out-dir ${OUT_HELLO}
	riscv32-unknown-elf-gcc ${LD_FLAGS} $(DBG) ${LIB_HELLO} ${LIB_LIBC} -o ${OUT_HELLO}/hello.elf

tmp_check:
	mkdir -p $(OUT_HELLO)

clean:
	rm -rf ${OUT_HELLO}

PHONY: tmp_check
