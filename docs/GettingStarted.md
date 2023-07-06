
## Getting started with repo & the build system.

CantripOS includes a multi-platform build framework.
This framwork leverages make, cmake, and cargo.
To get started follow these steps:

1. Clone the Sparrow project from GitHub using the
   [repo tool](https://gerrit.googlesource.com/git-repo/+/refs/heads/master/README.md)
   We assume below this lands in a top-level directory named "sparrow".
2. Download, build, and boot the system to the Cantrip shell prompt.
   For now the only target platform that works is "rpi3"
   (for a raspi3b machine running in simulation on qemu).

``` shell
mkdir sparrow
cd sparrow
repo init -u https://github.com/AmbiML/sparrow-manifest -m sparrow-manifest.xml
repo sync -j$(nproc)
export PLATFORM=rpi3
source build/setup.sh
m prereqs
m simulate
```

[Beware that if your repo tool is out of date you may need to supply `-b main`
to the init request as older versions of repo only check for a `master` branch.]

### Prerequisites.

Note the above assumes you have the follow prerequisites installed on your system
and **in your shell's search path**:
1. Gcc (or clang) for the target architecture
2. Rust; any nightly build >=nightly-2021-11-05 should work. A default version is set
   in the build/setup.sh script; if that is not what you are using either edit the shell
   script or export `CANTRIP_RUST_VERSION` in each shell where you work.
   Beware that we use various nightly-only features that are not supported by stable
   versions of Rust (e.g. to override the default TLS model).
3. Whichever simulator seL4 expects for your target architecture; e.g. for aarch64 this
   is qemu-system-aarch64.

The `m prereqs` step installs required python packages using the `pip`
tool. At the moment this mostly sets up a virtual environment for the
project.

Because Sparrow is a CAmkES project you also need
[CAmkES dependencies](https://docs.sel4.systems/projects/buildsystem/host-dependencies.html#camkes-build-dependencies).

### First time setup.

Sparrow uses [repo](https://gerrit.googlesource.com/git-repo/+/refs/heads/master/README.md)
to download and piece together Sparrow git repositories as well as dependent projects /
repositories such as [seL4](https://github.com/seL4).

``` shell
$ repo init -u https://github.com/AmbiML/sparrow-manifest -m sparrow-manifest.xml
Downloading Repo source from https://gerrit.googlesource.com/git-repo

repo has been initialized in <your-directory>/sparrow/
If this is not the directory in which you want to initialize repo, please run:
   rm -r <your-directory>/sparrow//.repo
and try again.
$ repo sync -j12
Fetching: 100% (23/23), done in 9.909s
Garbage collecting: 100% (23/23), done in 0.218s
Checking out: 100% (23/23), done in 0.874s
repo sync has finished successfully.
$ export PLATFORM=rpi3
$ export CANTRIP_RUST_VERSION=nightly
$ source build/setup.sh
========================================
ROOTDIR=/<your-directory>/sparrow
OUT=/<your-directory>/sparrow/out
PLATFORM=rpi3
PYTHON_SPARROW_ENV=<your-directory>/sparrow/cache/rpi3-venv
========================================

Type 'm [target]' to build.

Targets available are:

...
cantrip cantrip-build-debug-prepare cantrip-build-release-prepare cantrip-builtins
cantrip-builtins-debug cantrip-builtins-release cantrip-bundle-debug cantrip-bundle-release
cantrip-clean cantrip-clean-headers cantrip-clippy cantrip-component-headers
...
$ m prereqs
<your-directory>/sparrow/scripts/install-prereqs.sh \
        -p "<your-directory>/sparrow/scripts/python-requirements.txt \
                 " \
        -a ""
Installing apt package dependencies...
Installing python package dependencies...
Creating virtual python environment <your-directory>/sparrow/cache/rpi3-venv
...
Installation complete.
$ m simulate
...
info: component 'rust-std' for target 'aarch64-unknown-none' is up to date
loading initial cache file <your-directory>/sparrow/cantrip/projects/camkes/settings.cmake
-- Set platform details from PLATFORM=rpi3
--   KernelPlatform: bcm2837
--   KernelARMPlatform: rpi3
-- Setting from flags KernelSel4Arch: aarch64
-- Found seL4: <your-directory>/sparrow/kernel
-- The C compiler identification is GNU 11.2.1
...
[291/291] Generating images/capdl-loader-image-arm-bcm2837
...
qemu-system-aarch64 -machine raspi3b -nographic -serial null -serial mon:stdio -m size=1024M -s \
-kernel /<your-directory>/sparrow/out/cantrip/aarch64-unknown-elf/release/capdl-loader-image \
--mem-path /<your-directory>/sparrow/out/cantrip/aarch64-unknown-elf/release/cantrip.mem

ELF-loader started on CPU: ARM Ltd. Cortex-A53 r0p4
  paddr=[8bd000..fed0ff]
No DTB passed in from boot loader.
Looking for DTB in CPIO archive...found at 9b3ef8.
Loaded DTB from 9b3ef8.
   paddr=[23c000..23ffff]
ELF-loading image 'kernel' to 0
  paddr=[0..23bfff]
  vaddr=[ffffff8000000000..ffffff800023bfff]
  virt_entry=ffffff8000000000
ELF-loading image 'capdl-loader' to 240000
  paddr=[240000..4c0fff]
  vaddr=[400000..680fff]
  virt_entry=4009e8
Enabling MMU and paging
Jumping to kernel-image entry point...

Warning:  gpt_cntfrq 62500000, expected 19200000
Bootstrapping kernel
Booting all finished, dropped to user space
cantrip_os_rootserver::Bootinfo: (1969, 131072) empty slots 1 nodes (15, 83) untyped 131072 cnode slots
cantrip_os_rootserver::Model: 1821 objects 1 irqs 0 untypeds 2 asids
cantrip_os_rootserver::capDL spec: 0.39 Mbytes
cantrip_os_rootserver::CAmkES components: 5.85 Mbytes
cantrip_os_rootserver::Rootserver executable: 1.07 Mbytes
<<seL4(CPU 0) [decodeARMFrameInvocation/2137 T0xffffff80004c7400 "rootserver" @44373c]: ARMPageMap: Attempting to remap a frame that does not belong to the passed address space>>
...
<<seL4(CPU 0) [decodeCNodeInvocation/107 T0xffffff80009a3400 "rootserver" @4268a0]: CNode Copy/Mint/Move/Mutate: Source slot invalid or empty.>>
...
CANTRIP> builtins
autostart.repl 336
hello.app 1084
keyval.app 32276
logtest.app 26948
panic.app 25688
timer.app 33060
CANTRIP> install hello.app
cantrip_memory_manager::Global memory: 0 allocated 130543360 free, reserved: 2273280 kernel 1359872 user
Collected 1084 bytes of data, crc32 5b847193
Application "hello" installed
CANTRIP> start hello
Bundle "hello" started.
CANTRIP> install keyval.app

I am a C app!
Done, sleeping in WFI loop
Collected 32276 bytes of data, crc32 bcf05273
Application "keyval" installed
CANTRIP> start keyval
Bundle "keyval" started.
...
CANTRIP> mstats
48 bytes in-use, 130543312 bytes free, 720512 bytes requested, 1359872 overhead
2 objs in-use, 196 objs requested
CANTRIP> EOF
```

The `m simulate` command can be run repeatedly. If you need to reset
your setup just remove the build tree and re-run `m simulate`; e.g.

``` shell
$ cd sparrow
$ m clean
$ m simulate
```

### Build system: primer.

The setup procedure required:

``` shell
$ export PLATFORM=rpi3
$ export CANTRIP_RUST_VERSION=nightly  # force use of "nightly" channel
$ source build/setup.sh
```

This defined various shell functions/aliases for working with CantripOS. In particular
the `m` command is the primary mechanism for building and running simulations.
The default target command is `simulate` so these are equivalent:

``` shell
$ m simulate
$ m               # default target is simulate
```

As seen above, another useful target is `m simulate-debug` which builds a debug version
of the system and starts up a simulator. In this case the simulator (platform-dependent)
supports connecting GDB with scripts/kgdb.sh in a separate window/terminal. For more
information on using GDB with seL4 check [here](CantripGdbUserSpaceDebugging.md).

There is tab completion for build targets depending on your shell; e.g.

``` shell
$ m <TAB>
cargo_test_debugconsole_zmodem            cantrip-gen-headers
cargo_test_cantrip                           keyval_debug
cargo_test_cantrip_os_common_logger          keyval_release
cargo_test_cantrip_os_common_slot_allocator  logtest_debug
cargo_test_cantrip_proc_interface            logtest_release
cargo_test_cantrip_proc_manager              matcha_tock_clean
clean                                     matcha_tock_debug
collate_cantrip_rust_toolchain               matcha_tock_release
collate_matcha_rust_toolchain             minisel_debug
collate_rust_toolchains                   minisel_release
elfconvert                                multihart_boot_rom
fibonacci_debug                           multihart_boot_rom_clean
fibonacci_release                         panic_debug
flatbuffers                               panic_release
flatbuffers-clean                         prereqs
...
```

There is also a `hmm`
command that can display help information for a build target. For example,

``` shell
$ hmm sel4test

sel4test: (defined in build/platforms/sparrow/sim_sel4test.mk)
 C-based libsel4 syscall api wrappers. The result is run under Renode.
```

### Build system: multi-platform support.

The build system supports multiple target platforms. But at the moment there are
only two platforms--sparrow & rpi3--so this is less exciting. The current
platform is kept in your shell's environmnet so after a default setup you will see:

``` shell
$ source build/setup.sh
$ printenv PLATFORM
sparrow
```

But this also means that you need to `source build/setup.sh` in each shell
where you work on the software.

To switch the current platform use the `set-platform` shell function:

``` shell
$ set-platform rpi3
$ printenv PLATFORM
rpi3
```

There is also a `list-platforms` shell function that you can use in lieu of
tab completion with the `set-platform` command.

### Build system: cleaning build artifacts

Most of the time `m simulate` or `m simulate-debug` is all you need to do work:
make dependencies will cause only necessary operations to be done.
But sometimes it's necessary to remove build artifacts (e.g. because depeencies
are incorrect or the dependencies are overly conservative resulting in excessive
build steps).
Therre are many targets that selectively clear out unwanted artifacts but most
of the time you will just want to use:

``` shell
$ m cantrip-clean
```

which removes all build artifacts for the current platform, or

```shell
$ m clean
```

which removes all build artifacts for all platforms.

### [Next Section: CantripOS software organization](SourceCrates.md)
