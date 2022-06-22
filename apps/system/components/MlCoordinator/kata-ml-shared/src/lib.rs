#![no_std]
#![allow(dead_code)]

// Data structures used throughout the Cantrip ML implementation that do not
// depend on cantrip-os-common.

extern crate alloc;

use alloc::string::String;

/// An image is uniquely identified by the bundle that owns it and the
/// particular model id in that bundle.
#[derive(Clone, Eq, PartialEq)]
pub struct ImageId {
    pub bundle_id: String,
    pub model_id: String,
}

/// An image consists of five sections. See go/sparrow-vc-memory for a
/// description of each section. Sizes are in bytes.
#[derive(Clone, Default)]
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
}

// XXX: Out-dated and should use ImageSizes. Fix in merge.
/// The Vector Core uses a Windowed MMU (go/sparrow-wmmu) in order to prevent
/// models from interferring with each other. Before executing a model,
/// windows to only that model's code and data are opened.
/// A window is represented by an address and size of that window.
pub struct Window {
    pub addr: usize,
    pub size: usize,
}

/// When a model is loaded onto the Vector Core, the ML Coordinator needs to
/// track where each window is.
pub struct ModelSections {
    pub instructions: Window,
    pub data: Window,
}

/// The page size of the WMMU.
pub const WMMU_PAGE_SIZE: usize = 0x1000;

/// The maximum number of models that the MLCoordinator can handle. This is
/// bounded by timer slots. It's unlikely we'll be anywhere near this due to
/// memory contstraints.
pub const MAX_MODELS: usize = 32;

// TODO(b/214092253): ITCM size blow-up needs to be addressed.
pub const ITCM_SIZE: usize = 0x100000;
pub const ITCM_PADDR: usize = 0x32000000;
pub const DTCM_SIZE: usize = 0x1000000;
pub const DTCM_PADDR: usize = 0x34000000;

// XXX: Only TCM variables should remain after TCM merge. Temporarily keeping
// both until further merge work is complete.
pub const TCM_SIZE: usize = 0x1000000;
pub const TCM_PADDR: usize = 0x34000000;
