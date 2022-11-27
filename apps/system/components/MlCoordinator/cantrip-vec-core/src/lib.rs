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

//! cantrip-vec-core is the vector core driver. It is responsible for providing
//! convenient methods for interacting with the hardware.

extern crate alloc;

mod vc_top;

use cantrip_io::Read;
use cantrip_ml_shared::{OutputHeader, Permission, WindowId, TCM_PADDR, TCM_SIZE};
use core::mem::size_of;
use core::slice;
use log::{error, trace};

extern "C" {
    static TCM: *mut u32;
}

fn get_tcm_slice() -> &'static mut [u32] {
    unsafe { slice::from_raw_parts_mut(TCM, TCM_SIZE / size_of::<u32>()) }
}

pub fn enable_interrupts(enable: bool) {
    let intr_enable = vc_top::IntrEnable::new()
        .with_host_req(enable)
        .with_finish(enable)
        .with_instruction_fault(enable)
        .with_data_fault(enable);
    vc_top::set_intr_enable(intr_enable);
}

pub fn set_wmmu_window(
    window_id: WindowId,
    start_address: usize,
    length: usize,
    permission: Permission,
) {
    trace!(
        "Set window {:?} to addr {:x} len {:x}",
        window_id,
        start_address,
        length
    );
    vc_top::set_mmu_window_offset(window_id as usize, start_address);
    // The length of the window is not the size of the window, but rather
    // the last address of the window. This saves us a bit in hardware:
    // 0x400000 is 23 bits vs. 0x3FFFFF 22 bits.
    vc_top::set_mmu_window_length(window_id as usize, length - 1);
    vc_top::set_mmu_window_permission(window_id as usize, permission);
}

/// Start the core at the default PC.
pub fn run() {
    let ctrl = vc_top::Ctrl::new()
        .with_freeze(false)
        .with_vc_reset(false)
        .with_pc_start(0);
    vc_top::set_ctrl(ctrl);
}

/// Writes the section of the image from |start_address| to
/// |start_address + on_flash_size| into the TCM. Zeroes the section from
/// |on_flash_size| to |in_memory_size|. Returns None if the write failed.
pub fn write_image_part<R: Read>(
    image: &mut R,
    start_address: usize,
    on_flash_size: usize,
    in_memory_size: usize,
) -> Option<()> {
    let start = start_address - TCM_PADDR;

    trace!(
        "Writing {:x} bytes to 0x{:x}, {:x} unpacked size",
        on_flash_size,
        start_address,
        in_memory_size
    );

    let tcm_slice = unsafe { slice::from_raw_parts_mut(TCM as *mut u8, TCM_SIZE) };

    if let Err(e) = image.read_exact(&mut tcm_slice[start..start + on_flash_size]) {
        error!("Section read error {:?}", e);
        return None;
    };

    // TODO(jesionowski): Use hardware clear when TCM_SIZE fits into INIT_END.
    tcm_slice[start + on_flash_size..start + in_memory_size].fill(0x00);

    Some(())
}

/// Move |src_index..src_index + byte_length| to
/// |dest_index..dest_index + byte_length|.
pub fn tcm_move(src: usize, dest: usize, byte_length: usize) {
    trace!(
        "Moving 0x{:x} bytes to 0x{:x} from 0x{:x}",
        byte_length,
        dest as usize,
        src as usize,
    );

    let tcm_slice = get_tcm_slice();
    let src_index = (src - TCM_PADDR) / size_of::<u32>();
    let dest_index = (dest - TCM_PADDR) / size_of::<u32>();
    let count: usize = byte_length / size_of::<u32>();

    tcm_slice.copy_within(src_index..src_index + count, dest_index);
}

// Interrupts are write 1 to clear.
pub fn clear_host_req() {
    let mut intr_state = vc_top::get_intr_state();
    intr_state.set_host_req(true);
    vc_top::set_intr_state(intr_state);
}

pub fn clear_finish() {
    let mut intr_state = vc_top::get_intr_state();
    intr_state.set_finish(true);
    vc_top::set_intr_state(intr_state);
}

pub fn clear_instruction_fault() {
    let mut intr_state = vc_top::get_intr_state();
    intr_state.set_instruction_fault(true);
    vc_top::set_intr_state(intr_state);
}

pub fn clear_data_fault() {
    let mut intr_state = vc_top::get_intr_state();
    intr_state.set_data_fault(true);
    vc_top::set_intr_state(intr_state);
}

// TODO(jesionowski): Use when TCM_SIZE fits into INIT_END.
#[allow(dead_code)]
fn clear_section(start: u32, end: u32) {
    let init_start = vc_top::InitStart::new().with_address(start);
    vc_top::set_init_start(init_start);

    let init_end = vc_top::InitEnd::new().with_address(end).with_valid(true);
    vc_top::set_init_end(init_end);
}

/// Zeroes out |byte_length| bytes starting at |addr|.
pub fn clear_tcm(addr: usize, byte_length: usize) {
    assert!(addr >= TCM_PADDR);
    assert!(addr + byte_length <= TCM_PADDR + TCM_SIZE);

    trace!("Clearing 0x{:x} bytes at 0x{:x}", byte_length, addr);

    let start = (addr - TCM_PADDR) / size_of::<u32>();
    let count: usize = byte_length / size_of::<u32>();

    // TODO(jesionowski): Use clear_section method when able.
    let tcm_slice = get_tcm_slice();
    tcm_slice[start..start + count].fill(0x00);
}

// TODO(jesionowski): Use when TCM_SIZE fits into INIT_END.
// We'll want to kick off the hardware clear after the execution is complete,
// holding off the busy-wait until we're ready to start another execution.
#[allow(dead_code)]
pub fn wait_for_clear_to_finish() { while !vc_top::get_init_status().init_done() {} }

/// Transmutes a copy of the bytes at |addr| into an OutputHeader.
pub fn get_output_header(addr: usize) -> OutputHeader {
    assert!(addr >= TCM_PADDR);
    assert!(addr + size_of::<OutputHeader>() <= TCM_PADDR + TCM_SIZE);

    let offset: isize = (addr - TCM_PADDR).try_into().unwrap();

    unsafe {
        let ptr = TCM.offset(offset) as *const OutputHeader;
        *ptr
    }
}
