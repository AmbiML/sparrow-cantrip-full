INTERFACES=${OUT}/cantrip/components

${INTERFACES}/StorageManagerBindings.h: src/lib.rs cbindgen.toml
	cbindgen -c cbindgen.toml src/lib.rs -o $@
