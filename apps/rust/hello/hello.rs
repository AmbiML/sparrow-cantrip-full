/*
 * Copyright 2021, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#![no_std]
#![no_main]

extern crate libcantrip;
use cantrip_os_common::logger::CantripLogger;
use cantrip_sdk_interface::*;
use log::info;

// Message output is sent through the cantrip-os-logger which calls logger_log
// to deliver data to the console. Redict to the sdk.
#[no_mangle]
#[allow(unused_variables)]
pub fn logger_log(_level: u8, msg: *const cstr_core::c_char) {
    if let Ok(str) = unsafe { cstr_core::CStr::from_ptr(msg) }.to_str() {
        let _ = cantrip_sdk_log(str);
    }
}

#[no_mangle]
pub fn main() {
    // Setup logger; (XXX maybe belongs in the SDKRuntime)
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    match cantrip_sdk_ping() {
        Ok(_) => info!("ping!"),
        Err(e) => info!("cantrip_sdk_ping failed: {:?}", e),
    }
    info!("I am a Rust app, hear me log!");
    info!("Done, wimper ...");
}
