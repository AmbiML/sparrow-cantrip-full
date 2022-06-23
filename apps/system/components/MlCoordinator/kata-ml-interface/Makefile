INTERFACES=../../../interfaces

${INTERFACES}/MlCoordBindings.h: src/lib.rs cbindgen.toml
	cbindgen -c cbindgen.toml src/lib.rs -o $@
