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

#![allow(unused)]

//! Setters and getters for the Vector Core CSRs.

use cantrip_ml_shared::Permission;
use core::ptr;
use modular_bitfield::prelude::*;

extern "Rust" {
    fn get_csr() -> &'static [u8];
    fn get_csr_mut() -> &'static mut [u8];
}
fn get_csr_word() -> &'static [u32] { unsafe { core::mem::transmute(get_csr()) } }
fn get_csr_word_mut() -> &'static mut [u32] { unsafe { core::mem::transmute(get_csr_mut()) } }

#[bitfield]
pub struct IntrState {
    pub host_req: bool,
    pub finish: bool,
    pub instruction_fault: bool,
    pub data_fault: bool,
    #[skip]
    _unused: B28,
}

#[bitfield]
pub struct IntrEnable {
    pub host_req: bool,
    pub finish: bool,
    pub instruction_fault: bool,
    pub data_fault: bool,
    #[skip]
    _unused: B28,
}

#[bitfield]
pub struct IntrTest {
    pub host_req: bool,
    pub finish: bool,
    pub instruction_fault: bool,
    pub data_fault: bool,
    #[skip]
    _unused: B28,
}

#[bitfield]
pub struct Ctrl {
    pub freeze: bool,
    pub vc_reset: bool,
    pub pc_start: B17,
    #[skip]
    pub _unused0: B13,
}

#[bitfield]
pub struct MemoryBankCtrl {
    pub i_mem_enable: B4,
    pub d_mem_enable: B8,
    #[skip]
    pub _unused0: B20,
}

#[bitfield]
pub struct ErrorStatus {
    pub i_mem_out_of_range: bool,
    pub d_mem_out_of_range: bool,
    pub i_mem_disable_access: B4,
    pub d_mem_disable_access: B8,
    #[skip]
    pub _unused0: B18,
}

#[bitfield]
pub struct InitStart {
    pub address: B22,
    pub imem_dmem_sel: bool,
    #[skip]
    pub _unused0: B9,
}

#[bitfield]
pub struct InitEnd {
    pub address: B22,
    pub valid: bool,
    #[skip]
    pub _unused0: B9,
}

#[bitfield]
pub struct InitStatus {
    pub init_pending: bool,
    pub init_done: bool,
    #[skip]
    pub _unused0: B30,
}

// XXX TODO(sleffler): re-check address calculations

pub fn get_intr_state() -> IntrState {
    let ptr = ptr::addr_of!(get_csr_word()[0]);
    unsafe { IntrState::from_bytes(ptr::read_volatile(ptr).to_ne_bytes()) }
}

pub fn set_intr_state(intr_state: IntrState) {
    unsafe {
        get_csr_word_mut()[0] = u32::from_ne_bytes(intr_state.into_bytes());
    }
}

pub fn get_intr_enable() -> IntrEnable {
    let ptr = ptr::addr_of!(get_csr_word()[1]);
    unsafe { IntrEnable::from_bytes(ptr::read_volatile(ptr).to_ne_bytes()) }
}

pub fn set_intr_enable(intr_enable: IntrEnable) {
    unsafe {
        get_csr_word_mut()[1] = u32::from_ne_bytes(intr_enable.into_bytes());
    }
}

pub fn get_intr_test() -> IntrTest {
    let ptr = ptr::addr_of!(get_csr_word()[2]);
    unsafe { IntrTest::from_bytes(ptr::read_volatile(ptr).to_ne_bytes()) }
}

pub fn set_intr_test(intr_test: IntrTest) {
    unsafe {
        get_csr_word_mut()[2] = u32::from_ne_bytes(intr_test.into_bytes());
    }
}

pub fn get_ctrl() -> Ctrl {
    let ptr = ptr::addr_of!(get_csr_word()[3]);
    unsafe { Ctrl::from_bytes(ptr::read_volatile(ptr).to_ne_bytes()) }
}

pub fn set_ctrl(ctrl: Ctrl) {
    unsafe {
        get_csr_word_mut()[3] = u32::from_ne_bytes(ctrl.into_bytes());
    }
}

pub fn get_memory_bank_ctrl() -> MemoryBankCtrl {
    let ptr = ptr::addr_of!(get_csr_word()[4]);
    unsafe { MemoryBankCtrl::from_bytes(ptr::read_volatile(ptr).to_ne_bytes()) }
}

pub fn set_memory_bank_ctrl(memory_bank_ctrl: MemoryBankCtrl) {
    unsafe {
        get_csr_word_mut()[4] = u32::from_ne_bytes(memory_bank_ctrl.into_bytes());
    }
}

pub fn get_error_status() -> ErrorStatus {
    let ptr = ptr::addr_of!(get_csr_word()[5]);
    unsafe { ErrorStatus::from_bytes(ptr::read_volatile(ptr).to_ne_bytes()) }
}

pub fn set_error_status(error_status: ErrorStatus) {
    unsafe {
        get_csr_word_mut()[5] = u32::from_ne_bytes(error_status.into_bytes());
    }
}

pub fn get_init_start() -> InitStart {
    let ptr = ptr::addr_of!(get_csr_word()[6]);
    unsafe { InitStart::from_bytes(ptr::read_volatile(ptr).to_ne_bytes()) }
}

pub fn set_init_start(init_start: InitStart) {
    unsafe {
        get_csr_word_mut()[6] = u32::from_ne_bytes(init_start.into_bytes());
    }
}

pub fn get_init_end() -> InitEnd {
    let ptr = ptr::addr_of!(get_csr_word()[7]);
    unsafe { InitEnd::from_bytes(ptr::read_volatile(ptr).to_ne_bytes()) }
}

pub fn set_init_end(init_end: InitEnd) {
    unsafe {
        get_csr_word_mut()[7] = u32::from_ne_bytes(init_end.into_bytes());
    }
}

pub fn get_init_status() -> InitStatus {
    let ptr = ptr::addr_of!(get_csr_word()[8]);
    unsafe { InitStatus::from_bytes(ptr::read_volatile(ptr).to_ne_bytes()) }
}

pub fn set_init_status(init_status: InitStatus) {
    unsafe {
        get_csr_word_mut()[8] = u32::from_ne_bytes(init_status.into_bytes());
    }
}

// The WMMU registers start at 0x400 past the vector core CSRs and are 0x400
// long. Within the block, the registers are arranged like this:
// 0x0000: Window 0 Offset
// 0x0004: Window 0 Length
// 0x0008: Window 0 Permissions
// 0x000C: Unused
// 0x0010: Window 1 Offset
// 0x0014: Window 1 Length
// 0x0018: Window 1 Permissions
// 0x001C: Unused
// And so on.
const WMMU_OFFSET: usize = 0x400; // From base CSR.

const OFFSET_ADDR: usize = 0;
const LENGTH_ADDR: usize = 4;
const PERMISSIONS_ADDR: usize = 8;
const BYTES_PER_WINDOW: usize = 0x10;

const MAX_WINDOW: usize = 0x40;

unsafe fn window_ptr_mut(window: usize) -> *mut u8 {
    assert!(window < MAX_WINDOW, "Window out of range of WMMU");
    get_csr_mut()
        .as_mut_ptr()
        .add(WMMU_OFFSET + (window * BYTES_PER_WINDOW))
}

pub fn set_mmu_window_offset(window: usize, offset: usize) {
    unsafe {
        window_ptr_mut(window)
            .add(OFFSET_ADDR)
            .cast::<usize>()
            .write_volatile(offset);
    }
}

pub fn set_mmu_window_length(window: usize, length: usize) {
    unsafe {
        window_ptr_mut(window)
            .add(LENGTH_ADDR)
            .cast::<usize>()
            .write_volatile(length);
    }
}

pub fn set_mmu_window_permission(window: usize, permission: Permission) {
    unsafe {
        window_ptr_mut(window)
            .add(PERMISSIONS_ADDR)
            .cast::<usize>()
            .write_volatile(permission.bits() as usize);
    }
}
