// Auto-generated hardware structs from vc_top.hjson

#![allow(unused)]
use modular_bitfield::prelude::*;
use core::ptr;

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
