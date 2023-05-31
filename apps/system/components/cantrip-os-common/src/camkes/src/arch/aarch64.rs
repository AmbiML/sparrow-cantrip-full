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

// ARM aarch64 target support.

use static_assertions::assert_cfg;
assert_cfg!(target_arch = "aarch64");

// XXX TLS only has an *mut seL4_IPCBuffer but on aarch64 it requires at least 32 bytes
pub const CONFIG_SEL4RUNTIME_STATIC_TLS: usize = 32;

core::arch::global_asm!(
    "
    .section .text._camkes_start
    .align 3
    .globl _camkes_start
    .type _camkes_start, @function
_camkes_start:
    sub sp,sp,#16
    str x0, [sp]

    mov fp, #0
    mov lr, #0
    bl _camkes_start_rust
"
);

// NB: base should be mutable, cheat
pub unsafe fn set_tls_base(base: *const u8) {
    core::arch::asm!("msr tpidr_el0, x30", in("x30") base);
}
