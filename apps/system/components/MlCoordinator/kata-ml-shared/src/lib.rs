#![no_std]

// Data structures used throughout the Cantrip ML implementation that do not
// depend on cantrip-os-common.

extern crate alloc;

use alloc::string::String;
use bitflags::bitflags;

/// An image is uniquely identified by the bundle that owns it and the
/// particular model id in that bundle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageId {
    pub bundle_id: String,
    pub model_id: String,
}

/// An image consists of five sections. See go/sparrow-vc-memory for a
/// description of each section. Sizes are in bytes.
#[derive(Clone, Copy, Debug, Default)]
pub struct ImageSizes {
    pub text: usize,
    pub model_input: usize,
    pub model_output: usize,
    pub constant_data: usize,
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
}

/// The page size of the WMMU.
pub const WMMU_PAGE_SIZE: usize = 0x1000;

/// The maximum number of models that the MLCoordinator can handle. This is
/// bounded by timer slots. It's unlikely we'll be anywhere near this due to
/// memory contstraints.
pub const MAX_MODELS: usize = 32;

/// The size of the Vector Core's Tightly Coupled Memory (TCM).
pub const TCM_SIZE: usize = 0x1000000;

/// The address of the Vector Core's TCM, viewed from the SMC.
pub const TCM_PADDR: usize = 0x34000000;

// The virtualized address of each WMMU section (see: go/sparrow-vc-memory).
pub const TEXT_VADDR: usize = 0x80000000;
pub const CONST_DATA_VADDR: usize = 0x81000000;
pub const MODEL_OUTPUT_VADDR: usize = 0x82000000;
pub const STATIC_DATA_VADDR: usize = 0x83000000;
pub const MODEL_INPUT_VADDR: usize = 0x84000000;
pub const TEMP_DATA_VADDR: usize = 0x85000000;

#[derive(Clone, Copy, Debug)]
pub enum WindowId {
    Text = 0,
    ConstData = 1,
    ModelOutput = 2,
    StaticData = 3,
    ModelInput = 4,
    TempData = 5,
}

bitflags! {
    pub struct Permission: u32 {
        const READ    = 0b00000001;
        const WRITE   = 0b00000010;
        const EXECUTE = 0b00000100;
        const READ_WRITE = Self::READ.bits | Self::WRITE.bits;
        const READ_EXECUTE = Self::READ.bits | Self::EXECUTE.bits;
    }
}

pub fn round_up(a: usize, b: usize) -> usize {
    if (a % b) == 0 {
        a
    } else {
        usize::checked_add(a, b).unwrap() - (a % b)
    }
}
