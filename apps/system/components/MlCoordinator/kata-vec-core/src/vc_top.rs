#![allow(unused)]

// Setters and getters for the Vector Core CSRs.

use core::ptr;
use modular_bitfield::prelude::*;

extern "C" {
    static CSR: *mut [u32; 9];
}

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

pub fn get_intr_state() -> IntrState {
    unsafe { IntrState::from_bytes(ptr::read_volatile(CSR)[0].to_ne_bytes()) }
}

pub fn set_intr_state(intr_state: IntrState) {
    unsafe {
        (*CSR)[0] = u32::from_ne_bytes(intr_state.into_bytes());
    }
}

pub fn get_intr_enable() -> IntrEnable {
    unsafe { IntrEnable::from_bytes(ptr::read_volatile(CSR)[1].to_ne_bytes()) }
}

pub fn set_intr_enable(intr_enable: IntrEnable) {
    unsafe {
        (*CSR)[1] = u32::from_ne_bytes(intr_enable.into_bytes());
    }
}

pub fn get_intr_test() -> IntrTest {
    unsafe { IntrTest::from_bytes(ptr::read_volatile(CSR)[2].to_ne_bytes()) }
}

pub fn set_intr_test(intr_test: IntrTest) {
    unsafe {
        (*CSR)[2] = u32::from_ne_bytes(intr_test.into_bytes());
    }
}

pub fn get_ctrl() -> Ctrl { unsafe { Ctrl::from_bytes(ptr::read_volatile(CSR)[3].to_ne_bytes()) } }

pub fn set_ctrl(ctrl: Ctrl) {
    unsafe {
        (*CSR)[3] = u32::from_ne_bytes(ctrl.into_bytes());
    }
}

pub fn get_memory_bank_ctrl() -> MemoryBankCtrl {
    unsafe { MemoryBankCtrl::from_bytes(ptr::read_volatile(CSR)[4].to_ne_bytes()) }
}

pub fn set_memory_bank_ctrl(memory_bank_ctrl: MemoryBankCtrl) {
    unsafe {
        (*CSR)[4] = u32::from_ne_bytes(memory_bank_ctrl.into_bytes());
    }
}

pub fn get_error_status() -> ErrorStatus {
    unsafe { ErrorStatus::from_bytes(ptr::read_volatile(CSR)[5].to_ne_bytes()) }
}

pub fn set_error_status(error_status: ErrorStatus) {
    unsafe {
        (*CSR)[5] = u32::from_ne_bytes(error_status.into_bytes());
    }
}

pub fn get_init_start() -> InitStart {
    unsafe { InitStart::from_bytes(ptr::read_volatile(CSR)[6].to_ne_bytes()) }
}

pub fn set_init_start(init_start: InitStart) {
    unsafe {
        (*CSR)[6] = u32::from_ne_bytes(init_start.into_bytes());
    }
}

pub fn get_init_end() -> InitEnd {
    unsafe { InitEnd::from_bytes(ptr::read_volatile(CSR)[7].to_ne_bytes()) }
}

pub fn set_init_end(init_end: InitEnd) {
    unsafe {
        (*CSR)[7] = u32::from_ne_bytes(init_end.into_bytes());
    }
}

pub fn get_init_status() -> InitStatus {
    unsafe { InitStatus::from_bytes(ptr::read_volatile(CSR)[8].to_ne_bytes()) }
}

pub fn set_init_status(init_status: InitStatus) {
    unsafe {
        (*CSR)[8] = u32::from_ne_bytes(init_status.into_bytes());
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

fn window_addr(window: usize) -> usize {
    assert!(window < MAX_WINDOW, "Window out of range of WMMU");
    let mut addr: usize = unsafe { WMMU_OFFSET + CSR as usize };

    addr + window * BYTES_PER_WINDOW
}

pub fn set_mmu_window_offset(window: usize, offset: usize) {
    let addr = window_addr(window) + OFFSET_ADDR;
    unsafe {
        core::ptr::write_volatile(addr as *mut usize, offset);
    }
}

pub fn set_mmu_window_length(window: usize, length: usize) {
    let addr = window_addr(window) + LENGTH_ADDR;
    unsafe {
        core::ptr::write_volatile(addr as *mut usize, length);
    }
}

pub enum Permission {
    Read = 1,
    Write = 2,
    ReadWrite = 3,
    Execute = 4,
    ReadWriteExecute = 7,
}

pub fn set_mmu_window_permission(window: usize, permission: Permission) {
    let addr = window_addr(window) + PERMISSIONS_ADDR;
    unsafe {
        core::ptr::write_volatile(addr as *mut usize, permission as usize);
    }
}
