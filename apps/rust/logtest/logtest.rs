/*
 * Copyright 2022, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#![no_std]
#![no_main]

// Demo hookup of cantrip-os-logger to sdk_log (eventually move to libcantrip).

extern crate libcantrip;
use cantrip_os_common::allocator;
use cantrip_os_common::logger::CantripLogger;
use log::{error, info};
use sdk_interface::*;

// Message output is sent through the cantrip-os-logger which calls logger_log
// to deliver data to the console. Redirect to the sdk.
// TODO(sleffler): not being used for weak symbol ref in CantripLogger
#[no_mangle]
pub extern "C" fn logger_log(_level: u8, msg: *const cstr_core::c_char) {
    if let Ok(str) = unsafe { cstr_core::CStr::from_ptr(msg) }.to_str() {
        let _ = sdk_log(str);
    }
}

#[no_mangle]
pub fn main() {
    // Setup logger; (XXX maybe belongs in the SDKRuntime)
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    // NB: only need the allocator for error formatting.
    static mut HEAP: [u8; 4096] = [0; 4096];
    unsafe {
        allocator::ALLOCATOR.init(HEAP.as_mut_ptr() as _, HEAP.len());
    }

    match sdk_ping() {
        Ok(_) => info!("ping!"),
        Err(e) => error!("sdk_ping failed: {:?}", e),
    }
    let _ = sdk_log("DONE");
}
