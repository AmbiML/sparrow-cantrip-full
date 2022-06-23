INTERFACES=../../../interfaces

${INTERFACES}/TimerServiceBindings.h: src/lib.rs cbindgen.toml
	cbindgen -c cbindgen.toml src/lib.rs -o $@
