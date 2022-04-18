INTERFACES=../../../interfaces

${INTERFACES}/MemoryManagerBindings.h: src/lib.rs
	cbindgen -c cbindgen.toml $? -o $@
