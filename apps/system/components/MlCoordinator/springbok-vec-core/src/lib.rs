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

//! springbok-vec-core is the Springbok vector core driver. It is responsible
//! for providing convenient methods for interacting with the hardware.

#[allow(dead_code)]
mod vc_top;

extern crate alloc;
use cantrip_io::Read;
use cantrip_ml_interface::MlCoordError;
use cantrip_ml_shared::*;
use cantrip_proc_interface::BundleImage;
use core::mem::size_of;
use log::{error, info, trace, warn};

extern "Rust" {
    fn get_tcm() -> &'static [u8];
    fn get_tcm_mut() -> &'static mut [u8];
}
fn get_tcm_word_mut() -> &'static mut [u32] { unsafe { core::mem::transmute(get_tcm_mut()) } }

/// The page size of the WMMU.
pub const WMMU_PAGE_SIZE: usize = 0x1000;

/// The maximum number of models that the MLCoordinator can handle. This is
/// bounded by timer slots. It's unlikely we'll be anywhere near this due to
/// memory contstraints.
pub const MAX_MODELS: usize = 32;

/// The size of the Vector Core's Tightly Coupled Memory (TCM).
/// NB: this must match the MMIO region size specified to CAmkES by
///     TCM_size in MlCoordinator.camkes & system.camkes
pub use reg_constants::platform::TOP_MATCHA_VC_TOP_DMEM_SIZE_BYTES as TCM_SIZE;

/// The address of the Vector Core's TCM, viewed from the SMC.
/// NB: this is only used to calculate offsets into the MMIO region specified
///     to CAmkES; it is best to match TCM_paddr in MlCoordinator.camkes &
///     system.camkes but in theory a mismatch should not matter
pub use reg_constants::platform::TOP_MATCHA_VC_TOP_DMEM_BASE_ADDR as TCM_PADDR;

// The virtualized address of each WMMU section (see: go/sparrow-vc-memory).
pub const TEXT_VADDR: usize = 0x80000000;
pub const CONST_DATA_VADDR: usize = 0x81000000;
pub const MODEL_OUTPUT_VADDR: usize = 0x82000000;
pub const STATIC_DATA_VADDR: usize = 0x83000000;
pub const MODEL_INPUT_VADDR: usize = 0x84000000;
pub const TEMP_DATA_VADDR: usize = 0x85000000;

pub fn debug_state() {
    info!(target: "SPRINGBOK", "TCM {} @ {:#X}", TCM_SIZE, TCM_PADDR);
}

pub fn enable_interrupts(enable: bool) {
    vc_top::set_intr_enable(
        vc_top::IntrEnable::new()
            .with_host_req(enable)
            .with_finish(enable)
            .with_instruction_fault(enable)
            .with_data_fault(enable),
    );
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
    assert!(length > 0);
    vc_top::set_mmu_window_length(window_id as usize, length - 1);
    vc_top::set_mmu_window_permission(window_id as usize, permission);
}

/// Start the core at the default PC.
pub fn run() {
    vc_top::set_ctrl(
        vc_top::Ctrl::new()
            .with_freeze(false)
            .with_vc_reset(false)
            .with_pc_start(0),
    );
}

/// Scan the image for loadable sections and verify the image has a valid
/// format and fits into the TCM.
pub fn preprocess_image(id: &ImageId, image: &mut BundleImage) -> Option<(ImageSizes, ImageSizes)> {
    let mut on_flash_sizes = ImageSizes::default();
    let mut in_memory_sizes = ImageSizes::default();

    // TODO(sleffler): check magic
    while let Some(section) = image.next_section() {
        assert!(section.is_springbok());
        match section.vaddr {
            TEXT_VADDR => {
                on_flash_sizes.text = section.fsize;
                in_memory_sizes.text = round_up(section.msize, WMMU_PAGE_SIZE);
            }
            CONST_DATA_VADDR => {
                on_flash_sizes.constant_data = section.fsize;
                in_memory_sizes.constant_data = round_up(section.msize, WMMU_PAGE_SIZE);
            }
            MODEL_OUTPUT_VADDR => {
                on_flash_sizes.model_output = section.fsize;
                in_memory_sizes.model_output = round_up(section.msize, WMMU_PAGE_SIZE);
            }
            STATIC_DATA_VADDR => {
                on_flash_sizes.static_data = section.fsize;
                in_memory_sizes.static_data = round_up(section.msize, WMMU_PAGE_SIZE);
            }
            TEMP_DATA_VADDR => {
                on_flash_sizes.temporary_data = section.fsize;
                in_memory_sizes.temporary_data = round_up(section.msize, WMMU_PAGE_SIZE);
            }
            vaddr => {
                warn!("{}: skipping unexpected section at {:#x}", &id, vaddr);
            }
        }
    }
    if !in_memory_sizes.is_valid() {
        error!("{} invalid, section missing: {:?}", id, in_memory_sizes);
        return None;
    }
    if in_memory_sizes.total_size() > TCM_SIZE {
        error!("{} too big to fit in TCM: {:?}", id, in_memory_sizes);
        return None;
    }
    Some((on_flash_sizes, in_memory_sizes))
}

/// Writes the section of the image from |start_address| to
/// |start_address + on_flash_size| into the TCM. Zeroes the section from
/// |on_flash_size| to |in_memory_size|. Returns None if the write failed.
fn write_image_part<R: Read>(
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

    let tcm_slice = unsafe { get_tcm_mut() };

    if let Err(e) = image.read_exact(&mut tcm_slice[start..start + on_flash_size]) {
        error!("Section read error {:?}", e);
        return None;
    };

    // TODO(jesionowski): Use hardware clear when TCM_SIZE fits into INIT_END.
    tcm_slice[start + on_flash_size..start + in_memory_size].fill(0x00);

    Some(())
}

pub fn write_image(
    image: &mut BundleImage,
    mut temp_top: usize,
    on_flash_sizes: &ImageSizes,
    in_memory_sizes: &ImageSizes,
) -> Result<(), MlCoordError> {
    while let Some(section) = image.next_section() {
        // TODO(jesionowski): Ensure these are in order.
        if section.vaddr == TEXT_VADDR {
            write_image_part(image, temp_top, on_flash_sizes.text, in_memory_sizes.text)
                .ok_or(MlCoordError::LoadModelFailed)?;

            temp_top += in_memory_sizes.text;
        } else if section.vaddr == CONST_DATA_VADDR {
            write_image_part(
                image,
                temp_top,
                on_flash_sizes.constant_data,
                in_memory_sizes.constant_data,
            )
            .ok_or(MlCoordError::LoadModelFailed)?;

            temp_top += in_memory_sizes.constant_data;
        } else if section.vaddr == MODEL_OUTPUT_VADDR {
            // Don't load, but do skip.
            temp_top += in_memory_sizes.model_output;
        } else if section.vaddr == STATIC_DATA_VADDR {
            write_image_part(
                image,
                temp_top,
                on_flash_sizes.static_data,
                in_memory_sizes.static_data,
            )
            .ok_or(MlCoordError::LoadModelFailed)?;

            temp_top += in_memory_sizes.static_data;
        }
    }
    Ok(())
}

/// Move |src_index..src_index + byte_length| to
/// |dest_index..dest_index + byte_length|.
pub fn tcm_move(src: usize, dest: usize, byte_length: usize) {
    trace!("Moving {:#x} bytes to {:#x} from {:#x}", byte_length, dest, src);

    let tcm_slice = get_tcm_word_mut();
    let src_index = (src - TCM_PADDR) / size_of::<u32>();
    let dest_index = (dest - TCM_PADDR) / size_of::<u32>();
    let count: usize = byte_length / size_of::<u32>();

    tcm_slice.copy_within(src_index..src_index + count, dest_index);
}

// Interrupts are write 1 to clear.
pub fn clear_host_req() { vc_top::set_intr_state(vc_top::get_intr_state().with_host_req(true)); }
pub fn clear_finish() { vc_top::set_intr_state(vc_top::get_intr_state().with_finish(true)); }
pub fn clear_instruction_fault() {
    vc_top::set_intr_state(vc_top::get_intr_state().with_instruction_fault(true));
}
pub fn clear_data_fault() {
    vc_top::set_intr_state(vc_top::get_intr_state().with_data_fault(true));
}

// TODO(jesionowski): Use when TCM_SIZE fits into INIT_END.
#[allow(dead_code)]
fn clear_section(start: u32, end: u32) {
    vc_top::set_init_start(vc_top::InitStart::new().with_address(start));
    vc_top::set_init_end(vc_top::InitEnd::new().with_address(end).with_valid(true));
}

/// Zeroes out |byte_length| bytes starting at |addr|.
pub fn clear_tcm(addr: usize, byte_length: usize) {
    assert!(addr >= TCM_PADDR);
    assert!(addr + byte_length <= TCM_PADDR + TCM_SIZE);

    trace!("Clearing 0x{:x} bytes at 0x{:x}", byte_length, addr);

    let start = (addr - TCM_PADDR) / size_of::<u32>();
    let count: usize = byte_length / size_of::<u32>();

    // TODO(jesionowski): Use clear_section method when able.
    let tcm_slice = get_tcm_word_mut();
    tcm_slice[start..start + count].fill(0x00);
}

// TODO(jesionowski): Use when TCM_SIZE fits into INIT_END.
// We'll want to kick off the hardware clear after the execution is complete,
// holding off the busy-wait until we're ready to start another execution.
#[allow(dead_code)]
pub fn wait_for_clear_to_finish() { while !vc_top::get_init_status().init_done() {} }

/// Returns a copy of the OutputHeader.
pub fn get_output_header(data_top_addr: usize, sizes: &ImageSizes) -> OutputHeader {
    let addr = data_top_addr + sizes.model_output_offset();

    assert!(addr >= TCM_PADDR);
    assert!(addr + size_of::<OutputHeader>() <= TCM_PADDR + TCM_SIZE);
    assert!(((addr - TCM_PADDR) % size_of::<u32>()) == 0);

    unsafe {
        get_tcm()
            .as_ptr()
            .add(addr - TCM_PADDR)
            .cast::<OutputHeader>()
            .read()
    }
}
