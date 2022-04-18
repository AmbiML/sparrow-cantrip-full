INTERFACES=../../../interfaces

${INTERFACES}/SecurityCoordinatorBindings.h: src/lib.rs
	cbindgen -c cbindgen.toml $? -o $@
