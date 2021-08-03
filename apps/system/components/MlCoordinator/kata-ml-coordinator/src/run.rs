#![no_std]

// ML Coordinator Design Doc: go/sparrow-ml-doc

extern crate cantrip_panic;

use cantrip_logger::CantripLogger;
use log::debug;

static CANTRIP_LOGGER: CantripLogger = CantripLogger;

#[no_mangle]
pub extern "C" fn pre_init() {
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
}

#[no_mangle]
pub extern "C" fn run() {
    debug!("run");
}

// TODO: Move out of this file into separate (auto-generated?) file.
// TODO: Consider the modular_bitfield crate to represent bitfields.
fn vctop_ctrl(freeze: u32, vc_reset: u32, pc_start: u32) -> u32  {
    ((pc_start & 1) << 2) + ((vc_reset & 1) << 1) + freeze
}

#[no_mangle]
pub extern "C" fn mlcoord_execute() {
    // TODO: Call into MLCoordinator when available.
    // TODO: Once multiple model support is in start by name.
    // TODO: Read the fault registers after execution and report any errors found.
    extern "C" {
        fn vctop_set_ctrl(ctrl: u32);
    }
    unsafe {
        // Unhalt, start at default PC.
        vctop_set_ctrl(vctop_ctrl(0, 0, 0));
    }
}