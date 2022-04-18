INTERFACES=../../../interfaces

${INTERFACES}/StorageManagerBindings.h: src/lib.rs
	cbindgen -c cbindgen.toml $? -o $@
