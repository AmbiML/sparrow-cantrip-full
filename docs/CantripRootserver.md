
## CantripOS capDL rootserver application

The other main Rust piece of CantripOS is the rootserver application that is located in
*projects/capdl/cantrip-os-rootserver*. This depends on the *capdl* and *model*
submodules of *cantrip-os-common*. While it is possible to select either
cantrip-os-rootserver or the C-based capdl-loader-app with a CMake setting
in the CAmkES project's easy-settings.cmake file; e.g. `projects/cantrip/easy-settings.cmake` has:

```
#set(CAPDL_LOADER_APP "capdl-loader-app" CACHE STRING "")
set(CAPDL_LOADER_APP "cantrip-os-rootserver" CACHE STRING "")
```

using capdl-loader-app is not advised because it lacks important functionality
found only in cantrip-os-rootserver.

The most important differences between cantrip-os-rootserver and capdl-loader-app are:

- Support for reclaiming the rootserver's memory on exit.
- Support for CantripOS CAmkES features (e.g. MemoryManager, RTReply caps).
- Reduced memory consumption.

Otherwise cantrip-os-rootserver should provide the same functionality though
certain features are not tested (e.g. CONFIG_CAPDL_LOADER_STATIC_ALLOC)
and/or not well-tested (e.g. CONFIG_CAPDL_LOADER_CC_REGISTERS).

Beware that many of the cmake rootserver configuration parameters are not plumbed
through to the Rust code.  Its likely you will need to tweak features in the
Cargo.toml for cantrip-os-rootserver and/or cantrip-os-model (cantrip-os-common).

By default cantrip-os-rootserver prints information about the capDL specification
when it starts up. If you want verbose logging enable `LOG_DEBUG` or `LOG_TRACE`
in the Cargo.toml.

### [Next Section: Depending on CantripOS Rust crates](CrateDependencies.md)
