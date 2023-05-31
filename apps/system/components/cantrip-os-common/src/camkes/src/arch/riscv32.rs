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

// RISC-V 32-bit target support.

use static_assertions::assert_cfg;
assert_cfg!(target_arch = "riscv32");

use sel4_sys::seL4_IPCBuffer;

pub const CONFIG_SEL4RUNTIME_STATIC_TLS: usize = core::mem::size_of::<*const seL4_IPCBuffer>();

core::arch::global_asm!(
    "
    .section .text._camkes_start
    .align 2
    .globl _camkes_start
    .type _camkes_start, @function
_camkes_start:
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    addi sp,sp,-4
    sw a0, 0(sp)
    jal _camkes_start_rust
"
);

// NB: base should be mutable, cheat
pub unsafe fn set_tls_base(base: *const u8) {
    core::arch::asm!("or tp, a0, x0", in("a0") base);
}
