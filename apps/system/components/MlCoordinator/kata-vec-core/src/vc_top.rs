// Auto-generated hardware structs from vc_top.hjson

#![allow(unused)]
use core::ptr;
use modular_bitfield::prelude::*;

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

extern "C" {
    static csr: *mut [u32; 9];
}

pub fn get_intr_state() -> IntrState {
    unsafe { IntrState::from_bytes(ptr::read_volatile(csr)[0].to_ne_bytes()) }
}

pub fn set_intr_state(intr_state: IntrState) {
    unsafe {
        (*csr)[0] = u32::from_ne_bytes(intr_state.into_bytes());
    }
}

pub fn get_intr_enable() -> IntrEnable {
    unsafe { IntrEnable::from_bytes(ptr::read_volatile(csr)[1].to_ne_bytes()) }
}

pub fn set_intr_enable(intr_enable: IntrEnable) {
    unsafe {
        (*csr)[1] = u32::from_ne_bytes(intr_enable.into_bytes());
    }
}

pub fn get_intr_test() -> IntrTest {
    unsafe { IntrTest::from_bytes(ptr::read_volatile(csr)[2].to_ne_bytes()) }
}

pub fn set_intr_test(intr_test: IntrTest) {
    unsafe {
        (*csr)[2] = u32::from_ne_bytes(intr_test.into_bytes());
    }
}

pub fn get_ctrl() -> Ctrl {
    unsafe { Ctrl::from_bytes(ptr::read_volatile(csr)[3].to_ne_bytes()) }
}

pub fn set_ctrl(ctrl: Ctrl) {
    unsafe {
        (*csr)[3] = u32::from_ne_bytes(ctrl.into_bytes());
    }
}

pub fn get_memory_bank_ctrl() -> MemoryBankCtrl {
    unsafe { MemoryBankCtrl::from_bytes(ptr::read_volatile(csr)[4].to_ne_bytes()) }
}

pub fn set_memory_bank_ctrl(memory_bank_ctrl: MemoryBankCtrl) {
    unsafe {
        (*csr)[4] = u32::from_ne_bytes(memory_bank_ctrl.into_bytes());
    }
}

pub fn get_error_status() -> ErrorStatus {
    unsafe { ErrorStatus::from_bytes(ptr::read_volatile(csr)[5].to_ne_bytes()) }
}

pub fn set_error_status(error_status: ErrorStatus) {
    unsafe {
        (*csr)[5] = u32::from_ne_bytes(error_status.into_bytes());
    }
}

pub fn get_init_start() -> InitStart {
    unsafe { InitStart::from_bytes(ptr::read_volatile(csr)[6].to_ne_bytes()) }
}

pub fn set_init_start(init_start: InitStart) {
    unsafe {
        (*csr)[6] = u32::from_ne_bytes(init_start.into_bytes());
    }
}

pub fn get_init_end() -> InitEnd {
    unsafe { InitEnd::from_bytes(ptr::read_volatile(csr)[7].to_ne_bytes()) }
}

pub fn set_init_end(init_end: InitEnd) {
    unsafe {
        (*csr)[7] = u32::from_ne_bytes(init_end.into_bytes());
    }
}

pub fn get_init_status() -> InitStatus {
    unsafe { InitStatus::from_bytes(ptr::read_volatile(csr)[8].to_ne_bytes()) }
}

pub fn set_init_status(init_status: InitStatus) {
    unsafe {
        (*csr)[8] = u32::from_ne_bytes(init_status.into_bytes());
    }
}

// The WMMU registers start at 0x400 past the vector core CSRs. There are two
// blocks, one for the DMMU and one for the IMMU, each 0x400 long. Within the
// block, the registers are arranged like this:
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
const DMMU_OFFSET: usize = 0x400; // From IMMU CSRs.

const OFFSET_ADDR: usize = 0;
const LENGTH_ADDR: usize = 4;
const PERMISSIONS_ADDR: usize = 8;
const BYTES_PER_WINDOW: usize = 0x10;

const MAX_WINDOW: usize = 0x40;

fn window_addr(window: usize, is_immu: bool) -> usize {
    assert!(window < MAX_WINDOW, "Window out of range of WMMU");
    let mut addr: usize = unsafe { WMMU_OFFSET + csr as usize };

    if (!is_immu) {
        addr += DMMU_OFFSET;
    }

    addr + window * BYTES_PER_WINDOW
}

fn set_window_offset(window: usize, offset: usize, is_immu: bool) {
    let addr = window_addr(window, is_immu) + OFFSET_ADDR;
    unsafe {
        core::ptr::write_volatile(addr as *mut usize, offset);
    }
}

pub fn set_immu_window_offset(window: usize, offset: usize) {
    set_window_offset(window, offset, true);
}

pub fn set_dmmu_window_offset(window: usize, offset: usize) {
    set_window_offset(window, offset, false);
}

fn set_window_length(window: usize, length: usize, is_immu: bool) {
    let addr = window_addr(window, is_immu) + LENGTH_ADDR;
    unsafe {
        core::ptr::write_volatile(addr as *mut usize, length);
    }
}

pub fn set_immu_window_length(window: usize, length: usize) {
    set_window_length(window, length, true);
}

pub fn set_dmmu_window_length(window: usize, length: usize) {
    set_window_length(window, length, false);
}

pub enum Permission {
    Read = 1,
    Write = 2,
    ReadAndWrite = 3,
}

pub fn set_window_permission(window: usize, permission: Permission, is_immu: bool) {
    let addr = window_addr(window, is_immu) + PERMISSIONS_ADDR;
    unsafe {
        core::ptr::write_volatile(addr as *mut usize, permission as usize);
    }
}

pub fn set_immu_window_permission(window: usize, permission: Permission) {
    set_window_permission(window, permission, true);
}

pub fn set_dmmu_window_permission(window: usize, permission: Permission) {
    set_window_permission(window, permission, false);
}
