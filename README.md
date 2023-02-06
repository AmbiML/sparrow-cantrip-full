# Project Sparrow: CantripOS

Sparrow is a project to build a low-power secure embedded platform
for Ambient ML applications. The target platform leverages
[RISC-V](https://riscv.org/) and [OpenTitan](https://opentitan.org/).

The Sparrow
software includes a home-grown operating system named CantripOS, that runs
on top of [seL4](https://github.com/seL4) and (ignoring the seL4 kernel)
is written almost entirely in [Rust](https://www.rust-lang.org/).
CantripOS is comprised of a set of system services that are assembled
using CAmkES and applications that are dynamically loaded into a
constrained seL4 thread context and communicate with system services
through an SDK runtime environment.

The CAmkES project that assembles the CantripOS system services is
found in this git repository. It exists outside the seL4 source trees since it contains
code not intended to go to upstream seL4.

The target-platform-dependent CAmkES assembly description is found in
[apps/system/platforms](apps/system/platforms). It is built using the
[standard CAmkES build system](https://docs.sel4.systems/projects/camkes/manual.html#running-a-simple-example)
and requires the
[CAmkES dependencies](https://docs.sel4.systems/projects/buildsystem/host-dependencies.html#camkes-build-dependencies)
already be installed.
Top-level configuration is found in easy-settings.cmake and settings.cmake
with build-related configuration in build/cantrip.mk and nearby makefiles.

The following sections provide more in-depth documentation:

[Getting started](docs/GettingStarted.md)

[CantripOS software organization](docs/SourceCrates.md)

[Target architecture dependencies](docs/ArchitectureDependencies.md)

[Target platform dependencies](docs/PlatformDependencies.md)

[Testing](docs/Testing.md)

[Memory footprint](docs/MemoryFootprint.md)

[CantripOS capDL rootserver application](docs/CantripRootserver.md)

[Depending on CantripOS Rust crates](docs/CrateDependencies.md)

## Source Code Headers

Every file containing source code includes copyright and license
information. For dependent / non-Google code these are inherited from
the upstream repositories. If there are Google modifications you may find
the Google Apache license found below.

Apache header:

    Copyright 2022 Google LLC

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

        https://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
