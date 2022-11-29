
## CantripOS target architecture dependencies

There are four areas in CantripOS where target architecture-specific support is required:

- *sel4-sys*: system call wrappers
- *cantrip-os-model*: capDL support for the cantrip-os-rootserver
- *cantrip-proc-manager/sel4bundle*: application construction
- *libcantrip*: application runtime support

### sel4-sys

The sel4-sys crate provides interfaces to the seL4 kernel.
This is comprised of system call wrappers and related types & constants,
and support for state exported by the kernel during system startup
(e.g. the seL4_BootInfo provided by the kernel to the rootserver thread).

Much of sel4-sys's api's are generated at build time from XML specifications
in the seL4 kernel. Others are hardcoded by the crate.

The *arch* subdirectory holds code for each supported target archiecture:
aarch32 (ARM 32-bit), aarch64 (ARM 64-bit), riscv32 (RISC-V 32-bit),
riscv64 (RISC-V 64-bit), and x86* (not currently working and mostly ignored).
For example:

```shell
$ ls arch
aarch32_mcs.rs     aarch64_mcs.rs     arm_generic.rs     riscv32.rs         riscv64.rs        x86_generic.rs
aarch32_no_mcs.rs  aarch64_no_mcs.rs  riscv32_mcs.rs     riscv64_mcs.rs     riscv_generic.rs  x86.rs
aarch32.rs         aarch64.rs         riscv32_no_mcs.rs  riscv64_no_mcs.rs  x86_64.rs
```

The aarch64.rs file is included by the architecture-independent code.
It in turns includes either aarch64_mcs.rs or aarch64_no_mcs.rs depending
on whether the seL4 kernel is configured with or without MCS support.
The arm_generic.rs file has definitions that present architecture-specific
definitions and api's using an architecture-independent naming convention.
For example, every architecture has an seL4_SmallPageObject that maps
to their "small page" (4K on many). Similarly there is an seL4_Page_Map
call that maps to seL4_ARM_Page_Map on ARM systems.

To add a new architecture (or fix something like x86) follow the
patterns for the riscv and aarch architectures. Testing/validation of
the syscall wrappers is done using the [sel4test system](Testing.md)
(`m sel4test+wrappers`).

### cantrip-os-model

Support for the capDL "Model" resides in the cantrip-os-model crate.
Architecture-dependent support is mostly to setup an seL4 thread's virtual
address space (VSpace) and to create architecture-specific seL4 objects
that back IRQ's and I/O interfaces.  Like sel4-sys there is an *arch*
directory with architecture-specific support. Unlike sel4-sys MCS support
is orthogonal; that logic is split out to a *feature* subdirectory.

To add a new architecture (or fix something like x86) follow the pattern
for a working architecture. Testing/validation is done by running simple
CAmkES test cases under [cantrip-os-rootserver](CantripRootserver.md).

### cantrip-proc-manager/sel4bundle

The cantrip-proc-manager/sel4bundle module is similar to cantrip-os-model
except it constructs a CantripOS appllication from an sel4BundleImage
instead of a capDL Model. The realized seL4 thread may be limited in size
(e.g. on aarch64 a VSpace is constructed to support at most 2MiB of virtual
address space).
and has a fixed set of capabilties/objects provided to it.
Like cantrip-os-model there are *arch* and *feature* directories.

To add a new architecture follow the pattern for aarch64 or riscv32.
Testing is non-trivial with only printf-style debugging available unless
the simulator for the target-architecture supports GDB.

### libcantrip

libbcantrip is the support code statically linked into each CantripOS
application. There is a libcantrip crate for Rust applications and a
library for C applications. Each version has an *arch* subdirectory
with a crt0.S file that handles startup work for an application.
The crt0 code is tightly coupled to the sel4bundle setup work and to
SDKRuntime/sdk-interface crate that implements RPC communication
between applications and the SDKRuntime.
Rather than provide a (potentially) stale explanation of how this
works, consult the code.

### [Next Section: Target Platform dependencies](PlatformDependencies.md)
