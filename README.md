# Project Sparrow: CantripOS

Sparrow is a project to build a low-power secure embeded platform
for Ambient ML applications. The target platform leverages
[RISC-V](https://riscv.org/) and [OpenTitan](https://opentitan.org/).

The Sparrow
software includes a home-grown operating system named CantripOS, that runs
on top of [seL4](https://github.com/seL4) and (ignoring the seL4 kernel)
is written almost entirely in [Rust](https://www.rust-lang.org/).

This is a CAmkES project that assembles the entire CantripOS. It exists outside
the seL4 source trees since it contains code not intended to go to upstream
seL4.

This uses the [standard CAmkES build system](https://docs.sel4.systems/projects/camkes/manual.html#running-a-simple-example)
by symlinking CMakeLists.txt. It also symlinks settings.cmake, and so retains
the notion of "apps," which enables the build system to switch which assembly
it builds using the CAMKES\_APP CMake cache value. CantripOS just has one app,
*system*.
