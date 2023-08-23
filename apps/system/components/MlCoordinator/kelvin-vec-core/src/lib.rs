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

//! kelvin-vec-core is the Kelvin vector core driver. It is responsible
//! for providing convenient methods for interacting with the hardware.

#[allow(dead_code)]
mod ml_top;

use cantrip_io::Read;
use cantrip_ml_interface::MlCoordError;
use cantrip_ml_interface::MAX_OUTPUT_DATA;
use cantrip_ml_shared::*;
use cantrip_proc_interface::BundleImage;
use core::cmp;
use core::mem::size_of;
use log::{error, info, trace};

extern "Rust" {
    fn get_tcm() -> &'static [u8];
    fn get_tcm_mut() -> &'static mut [u8];
}
fn get_tcm_word_mut() -> &'static mut [u32] { unsafe { core::mem::transmute(get_tcm_mut()) } }

/// The maximum number of models that the MLCoordinator can handle. This is
/// bounded by timer slots. It's unlikely there will be anywhere near this
/// number due to memory contstraints.
pub const MAX_MODELS: usize = 4;

// XXX hack to satisfy cantrip-ml-support (forces alignment to 64 bits)
pub const WMMU_PAGE_SIZE: usize = 8;

/// The size of the Vector Core's Tightly Coupled Memory (TCM).
/// NB: this must match the MMIO region size specified to CAmkES by
///     TCM_size in MlCoordinator.camkes & system.camkes
pub use reg_constants::platform::TOP_MATCHA_ML_TOP_DMEM_SIZE_BYTES as TCM_SIZE;

/// The address of the Vector Core's TCM, viewed from the SMC.
/// NB: this is only used to calculate offsets into the MMIO region specified
///     to CAmkES; it is best to match TCM_paddr in MlCoordinator.camkes &
///     system.camkes but in theory a mismatch should not matter
pub use reg_constants::platform::TOP_MATCHA_ML_TOP_DMEM_BASE_ADDR as TCM_PADDR;

pub fn debug_state() {
    info!(target: "KELVIN", "TCM {} @ {:#X}", TCM_SIZE, TCM_PADDR);
}

pub fn enable_interrupts(enable: bool) {
    trace!("ENABLE {}", enable);
    let intr_enable = ml_top::IntrEnable::new()
        .with_host_req(enable)
        .with_finish(enable)
        .with_instruction_fault(enable);
    ml_top::set_intr_enable(intr_enable);
}

/// Start the core at the default PC.
pub fn run() {
    let pc: u64 = 0;
    trace!("RUN {:#x}", pc);
    let ctrl = ml_top::Ctrl::new()
        .with_freeze(false)
        .with_ml_reset(false)
        .with_pc_start(pc as u32);
    ml_top::set_ctrl(ctrl);
}

/*
 * From the kelvin linker script.
 *
 *  TCM_ORIGIN          --->   +=====================+
 *                             |                     |
 *                             |       .text         |
 *                             +---------------------|
 *                             |       .crt          |
 *                             +---------------------+
 *                             |      .rodata        |
 *                             +---------------------+
 *                             |    .init_array      |
 *                             +---------------------+
 *                             |       .data         |
 *                             +---------------------+
 *                             |       .bss          |
 *                             +---------------------+
 *                             |       .heap         |
 *                             |  (All unclamied     |
 *                             |       memory)       |
 *                             |                     |
 *  (TCM_END - stack    --->   +---------------------+
 *     - model_output          |       .stack        |
 *     - output_header)        +---------------------+
 *                             |   .model_output     |
 *  output_header (64B) --->   +---------------------+
 *                             |   .output_header    |
 *  TCM_END             --->   +=====================+
 */
pub fn preprocess_image(id: &ImageId, image: &mut BundleImage) -> Option<(ImageSizes, ImageSizes)> {
    let mut on_flash_sizes = ImageSizes::default();
    let mut in_memory_sizes = ImageSizes::default();

    // Kelvin workloads are unsegmented (no need/value w/o WMMU).
    while let Some(section) = image.next_section() {
        assert!(section.is_kelvin());
        on_flash_sizes.text += section.fsize;
        in_memory_sizes.text += round_up(section.msize, section.align);
    }
    if in_memory_sizes.text == 0 {
        error!("{} invalid, missing loadable section", id);
        return None;
    }
    if in_memory_sizes.total_size() > TCM_SIZE {
        error!("{} too big to fit in TCM: {:?}", id, in_memory_sizes);
        return None;
    }

    // XXX align sections
    // XXX do any of these section sizes come from original elf?
    // NB: calculate the output_header offset needed by ImageSizes::model_output_offset
    //    in_memory_sizes.constant_data = TCM_SIZE - 64;
    // NB: beware of setting model_output correctly as it is included in data_top_size().
    //    in_memory_sizes.model_output = 64;
    //    in_memory_sizes.stack = 16*1024;
    // Heap is all unclaimed memory between the loaded segment(s) + the stack.
    //    in_memory_sizes.heap = TCM_SIZE - (64 + in_memory_sizes.model_output + in_memory_sizes.stack);
    Some((on_flash_sizes, in_memory_sizes))
}

/// Writes the section of the image from |start_address| to
/// |start_address + on_flash_size| into the TCM. Zeroes the section from
/// |on_flash_size| to |in_memory_size|. Returns None if the write failed.
#[allow(dead_code)]
fn write_image_part<R: Read>(
    image: &mut R,
    start_address: usize,
    on_flash_size: usize,
    in_memory_size: usize,
) -> Option<()> {
    let start = start_address - TCM_PADDR;

    trace!(
        "WRITE {} bytes at {:#x}, {} unpacked size",
        on_flash_size,
        start_address,
        in_memory_size
    );

    let tcm_slice = unsafe { get_tcm_mut() };

    if let Err(e) = image.read_exact(&mut tcm_slice[start..start + on_flash_size]) {
        error!("Section read error {:?}", e);
        return None;
    };

    tcm_slice[start + on_flash_size..start + in_memory_size].fill(0);

    Some(())
}

pub fn write_image(
    image: &mut BundleImage,
    mut temp_top: usize,
    _on_flash_sizes: &ImageSizes,
    _in_memory_sizes: &ImageSizes,
) -> Result<(), MlCoordError> {
    while let Some(section) = image.next_section() {
        // XXX re-check file type?
        write_image_part(image, temp_top, section.fsize, section.msize)
            .ok_or(MlCoordError::LoadModelFailed)?;
        temp_top += section.msize;
    }
    Ok(())
}

/// Move |src_index..src_index + byte_length| to
/// |dest_index..dest_index + byte_length|.
pub fn tcm_move(src: usize, dest: usize, byte_length: usize) {
    trace!("COPY {} bytes {:#x} -> {:#x}", byte_length, src, dest);

    let tcm_slice = get_tcm_word_mut();
    let src_index = (src - TCM_PADDR) / size_of::<u32>();
    let dest_index = (dest - TCM_PADDR) / size_of::<u32>();
    let count: usize = byte_length / size_of::<u32>();

    tcm_slice.copy_within(src_index..src_index + count, dest_index);
}

/// Copy |src..src + src_len| to |dest|.
/// If |src| is out of range the copy is not done.
/// if |src_len| extends past the end of TCM or |dest| is
/// too small the copy is truncated to fit.
pub fn tcm_read(src: usize, src_len: usize, dest: &mut [u8; MAX_OUTPUT_DATA]) {
    trace!("READ {} bytes from {:#x}", src_len, src);

    if !(TCM_PADDR <= src && src < TCM_PADDR + TCM_SIZE) {
        trace!("READ skipped: invalid src address {:#x}", src);
        return;
    }
    let tcm_offset = src - TCM_PADDR;
    let count = cmp::min(cmp::min(src_len, TCM_SIZE - tcm_offset), dest.len());

    dest[..count].copy_from_slice(unsafe { &get_tcm_mut()[tcm_offset..tcm_offset + count] });
}

// Interrupts are write 1 to clear.
pub fn clear_host_req() { ml_top::set_intr_state(ml_top::get_intr_state().with_host_req(true)); }

pub fn clear_finish() { ml_top::set_intr_state(ml_top::get_intr_state().with_finish(true)); }

pub fn clear_instruction_fault() {
    ml_top::set_intr_state(ml_top::get_intr_state().with_instruction_fault(true));
}

pub fn reset() { ml_top::set_ctrl(ml_top::Ctrl::new().with_ml_reset(true)); }

/// Zeroes out |byte_length| bytes starting at |addr|.
pub fn clear_tcm(addr: usize, byte_length: usize) {
    trace!("CLEAR TCM {:#x} to {:#x}", addr, addr + byte_length);

    assert!(addr >= TCM_PADDR);
    assert!(addr + byte_length <= TCM_PADDR + TCM_SIZE);

    let start = (addr - TCM_PADDR) / size_of::<u32>();
    let count: usize = byte_length / size_of::<u32>();

    let tcm_slice = get_tcm_word_mut();
    tcm_slice[start..start + count].fill(0);
}

#[repr(C)]
struct KelvinOutputHeader {
    return_code: u32,
    output_ptr: u32,
    output_length: u32,
}

/// Returns a copy of the OutputHeader.
pub fn get_output_header(_data_top_addr: usize, _sizes: &ImageSizes) -> OutputHeader {
    // The OutputHeader is at a fixed location set in the linker script.
    // The output_ptr field points to indirect data if output_length > 0.
    let addr = TCM_PADDR + (TCM_SIZE - 64);
    trace!("GET OUTPUT at {:#x}", addr);
    assert!(((addr - TCM_PADDR) % size_of::<u32>()) == 0);

    let kelvin_header = unsafe {
        get_tcm()
            .as_ptr()
            .add(addr - TCM_PADDR)
            .cast::<KelvinOutputHeader>()
            .read()
    };
    OutputHeader {
        return_code: kelvin_header.return_code,
        // NB: output_ptr is a TCM offset, adjust for use with tcm_read
        output_ptr: Some((TCM_PADDR as u32) + kelvin_header.output_ptr),
        output_length: kelvin_header.output_length,
        epc: None,
    }
}
