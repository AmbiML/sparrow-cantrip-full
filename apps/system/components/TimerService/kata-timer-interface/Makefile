INTERFACES=../../../interfaces

${INTERFACES}/TimerServiceBindings.h: src/lib.rs
	cbindgen -c cbindgen.toml $? -o $@
