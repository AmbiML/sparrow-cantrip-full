#![no_std]

// fake-vec-core is a stubbed out version of cantrip-vec-core.

use cantrip_memory_interface::ObjDescBundle;
use cantrip_ml_interface::{ModelSections, Window};

pub fn enable_interrupts(_enable: bool) {}

pub fn set_wmmu(_sections: &ModelSections) {}

// NB: this function will be moved out of *-vec-core shortly.
pub fn load_image(_frames: &ObjDescBundle) -> Result<ModelSections, &'static str> {
    Ok(ModelSections {
        instructions: Window { addr: 0, size: 0 },
        data: Window { addr: 0, size: 0 },
    })
}

pub fn run() {}

pub fn clear_host_req() {}

pub fn clear_finish() {}

pub fn clear_instruction_fault() {}

pub fn clear_data_fault() {}

pub fn clear_tcm() {}

pub fn get_return_code() -> u32 {
    0
}

pub fn get_fault_register() -> u32 {
    0
}
