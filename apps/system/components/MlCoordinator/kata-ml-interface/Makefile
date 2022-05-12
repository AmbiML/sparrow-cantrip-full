INTERFACES=../../../interfaces

${INTERFACES}/MlCoordBindings.h: src/lib.rs
	cbindgen -c cbindgen.toml $? -o $@
