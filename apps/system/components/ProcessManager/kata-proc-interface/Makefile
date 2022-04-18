INTERFACES=../../../interfaces

${INTERFACES}/ProcessManagerBindings.h: src/lib.rs
	cbindgen -c cbindgen.toml $? -o $@
