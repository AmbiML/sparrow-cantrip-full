# Cantrip OS

This is a CAmkES project that assembles the entire Cantrip OS. It exists outside
the seL4 source trees, since it contains code not intended to go to upstream
seL4.

This uses the [standard CAmkES build system](https://docs.sel4.systems/projects/camkes/manual.html#running-a-simple-example)
by symlinking CMakeLists.txt. It also symlinks settings.cmake, and so retains
the notion of "apps," which enables the build system to switch which assembly
it builds using the CAMKES\_APP CMake cache value. Cantrip OS just has one app,
*system*.
