
## Target platform dependencies

The sel4-config mechanism for importing the seL4 kernel configuration
only dynamically sets features when ***compiling*** Rust code.
Target-platform dependencies like device drivers are handled by passing a
top-level feature through the cargo command line to effect the cargo dependency process.
For example, in DebugConsole/cantrip-debug-console/Cargo.toml configuration of the
platform-specific UART support for the command line interpreter is done with:

```
[features]
default = [
    "autostart_support",
]
autostart_support = ["default-uart-client"]
# Target platform support
CONFIG_PLAT_BCM2837 = []
CONFIG_PLAT_SPARROW = ["cantrip-uart-client"]
```

The CONFIG_PLAT_* features mirror the seL4 kernel config parameters and can be
used to select an optional dependency:

```
[dependencies]
...
default-uart-client = { path = "../default-uart-client", optional = true }
...
cantrip-uart-client = { path = "../cantrip-uart-client", optional = true }
```

The platform feature is injected into the build process in build/cantrip.mk with:

```
cmake ... -DRUST_GLOBAL_FEATURES=${CONFIG_PLATFORM} ...
```

In addition to including platform-dependencies in the build process they
may also need to be included in the CAmkES assembly; this done by having
the *system.camkes* file platform-specific.
For example, platforms/sparrow/syste.camkes plumbs the OpenTitanUARTDriver,
MlCoordinator, MailboxDriver, and TimerService components.

Some system services like the SDKRuntime are prepared for conditional inclusion
of dependent services;
e.g. if no MlCoordinator seervice is present all model-related SDK calls
returns SDKError::NoPlatformSupport.
This is done so applications have a stable ABI.

### [Next Section: Testing](Testing.md)
