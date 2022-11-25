/*
 * Copyright 2022, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#![no_std]
#![no_main]

// Demo hookup of cantrip-os-logger to sdk_log (eventually move to libcantrip).

use libcantrip::sdk_init;
use log::{error, info};
use sdk_interface::*;

#[no_mangle]
pub fn main() {
    // NB: need the allocator for error formatting.
    static mut HEAP: [u8; 4096] = [0; 4096];
    sdk_init(unsafe { &mut HEAP });

    match sdk_ping() {
        Ok(_) => info!("ping!"),
        Err(e) => error!("sdk_ping failed: {:?}", e),
    }
    let _ = sdk_log("DONE");
}
