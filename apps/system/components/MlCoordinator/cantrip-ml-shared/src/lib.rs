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

// Data structures used throughout the Cantrip ML implementation that do not
// depend on cantrip-os-common.

extern crate alloc;

use alloc::fmt;
use alloc::string::String;

/// An image is uniquely identified by the bundle that owns it and the
/// particular model id in that bundle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageId {
    pub bundle_id: String,
    pub model_id: String,
}
impl fmt::Display for ImageId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.bundle_id, self.model_id)
    }
}

/// An image consists of five sections. See go/sparrow-vc-memory for a
/// description of each section. Sizes are in bytes.
#[derive(Clone, Copy, Debug, Default)]
pub struct ImageSizes {
    pub model_input: usize,
    pub text: usize,
    pub constant_data: usize,
    pub model_output: usize,
    pub static_data: usize,
    pub temporary_data: usize,
}

impl ImageSizes {
    // Returns the sum of sections that are loaded as a contiguous segment.
    pub fn data_top_size(&self) -> usize {
        self.text + self.model_output + self.constant_data + self.static_data
    }

    pub fn total_size(&self) -> usize {
        self.data_top_size() + self.temporary_data + self.model_input
    }

    // A set of sizes is considered valid if everything but model_input is
    // non-zero.
    pub fn is_valid(&self) -> bool {
        // TODO(jesionowski): Add `&& self.model_output != 0` when model output
        // is integrated with model code.
        self.text != 0
            && self.constant_data != 0
            && self.static_data != 0
            && self.temporary_data != 0
    }

    pub fn model_output_offset(&self) -> usize { self.text + self.constant_data }
}

/// WMMU definitions (currently used only for Springbok).

#[derive(Clone, Copy, Debug)]
pub enum WindowId {
    Text = 0,
    ConstData = 1,
    ModelOutput = 2,
    StaticData = 3,
    ModelInput = 4,
    TempData = 5,
}

bitflags::bitflags! {
    pub struct Permission: u32 {
        const READ    = 0b00000001;
        const WRITE   = 0b00000010;
        const EXECUTE = 0b00000100;
        const READ_WRITE = Self::READ.bits | Self::WRITE.bits;
        const READ_EXECUTE = Self::READ.bits | Self::EXECUTE.bits;
    }
}

/// After execution our ML executable populates the top of .model_output with
/// the return code, the address of the fault if the RC is non-zero, and the
/// length of the output that follows.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct OutputHeader {
    pub return_code: u32,
    pub epc: u32,
    pub output_length: u32,
}

pub fn round_up(a: usize, b: usize) -> usize {
    if (a % b) == 0 {
        a
    } else {
        usize::checked_add(a, b).unwrap() - (a % b)
    }
}
