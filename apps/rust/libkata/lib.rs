// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![no_std]
#![allow(non_upper_case_globals)]
#![feature(global_asm)]
#![feature(thread_local)]

use core::arch::global_asm;
use sel4_sys::seL4_IPCBuffer;
use static_assertions::*;

// NB: this mimics the logic in build.rs
assert_cfg!(any(
    all(target_arch = "arm", target_pointer_width = "32"),
    all(target_arch = "aarch64"),
    all(target_arch = "riscv32"),
    all(target_arch = "riscv64"),
    all(target_arch = "x86"),
    all(target_arch = "x86_64"),
));

#[cfg(target_arch = "x86")]
global_asm!(include_str!("arch/x86/crt0.S"));

#[cfg(target_arch = "x86_64")]
global_asm!(include_str!("arch/x86_64/crt0.S"));

#[cfg(all(target_arch = "arm", target_pointer_width = "32"))]
global_asm!(include_str!("arch/aarch32/crt0.S"));

#[cfg(target_arch = "aarch64")]
global_asm!(include_str!("arch/aarch64/crt0.S"));

#[cfg(target_arch = "riscv32")]
global_asm!(include_str!("arch/riscv32/crt0.S"));

#[cfg(target_arch = "riscv64")]
global_asm!(include_str!("arch/riscv64/crt0.S"));

#[no_mangle]
#[thread_local]
static mut __sel4_ipc_buffer: *mut seL4_IPCBuffer = 0 as _;
