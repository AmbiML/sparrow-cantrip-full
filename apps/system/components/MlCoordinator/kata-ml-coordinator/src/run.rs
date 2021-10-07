#![no_std]

// ML Coordinator Design Doc: go/sparrow-ml-doc

extern crate cantrip_panic;

use cantrip_logger::CantripLogger;
use log::trace;

#[no_mangle]
pub extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
}

#[no_mangle]
pub extern "C" fn mlcoord__init() {
    // TODO(sleffler): maybe not needed?
    trace!("init");
}

// TODO: Move out of this file into separate (auto-generated?) file.
// TODO: Consider the modular_bitfield crate to represent bitfields.
fn vctop_ctrl(freeze: u32, vc_reset: u32, pc_start: u32) -> u32 {
    (pc_start << 2) + ((vc_reset & 1) << 1) + (freeze & 1)
}

#[no_mangle]
pub extern "C" fn mlcoord_execute() {
    // TODO: Call into MLCoordinator when available.
    // TODO: Once multiple model support is in start by name.
    extern "C" {
        fn vctop_set_ctrl(ctrl: u32);
    }
    unsafe {
        // Unhalt, start at default PC.
        vctop_set_ctrl(vctop_ctrl(0, 0, 0));
    }
}

#[no_mangle]
pub extern "C" fn vctop_return_update_result(return_code: u32, fault: u32) {
    // TODO(hcindyl): check the return code and fault registers, move the result
    // from TCM to SRAM, update the input/model, and call mlcoord_execute again.
    trace!(
        "vctop execution done with code {}, fault pc: {:#010X}",
        return_code,
        fault);
}
