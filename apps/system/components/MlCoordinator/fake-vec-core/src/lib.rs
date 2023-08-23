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

// fake-vec-core is a stubbed out version of springbok-vec-core.

extern crate alloc;
use alloc::boxed::Box;
use cantrip_io::Read;
use cantrip_ml_shared::*;

pub const WMMU_PAGE_SIZE: usize = 0x1000;
pub const MAX_MODELS: usize = 32;
pub const TCM_PADDR: usize = 0x34000000;
pub const TCM_SIZE: usize = 0x1000000;

pub fn enable_interrupts(_enable: bool) {}

pub fn set_wmmu_window(
    _window_id: WindowId,
    _start_address: usize,
    _length: usize,
    _permission: Permission,
) {
}

pub fn run() {}

pub fn write_image_part(
    _image: &mut Box<dyn Read>,
    _start_address: usize,
    _on_flash_size: usize,
    _unpacked_size: usize,
) -> Result<(), &'static str> {
    Ok(())
}

pub fn tcm_move(_src_offset: usize, _dest_offset: usize, _byte_length: usize) {}

pub fn clear_host_req() {}

pub fn clear_finish() {}

pub fn clear_instruction_fault() {}

pub fn clear_data_fault() {}

pub fn reset() {}

pub fn clear_tcm(_addr: usize, _len: usize) {}

pub fn wait_for_clear_to_finish() {}

pub fn get_output_header(_data_top_addr: usize, _sizes: &ImageSizes) -> OutputHeader {
    OutputHeader::default()
}
