/*
 * Copyright 2021, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#![no_std]
#![no_main]

extern crate alloc;
use alloc::string::String;
use libcantrip::sdk_init;
use log::{error, info};
use sdk_interface::*;
use SDKRuntimeError::*;

#[no_mangle]
pub fn main() {
    static mut HEAP: [u8; 4096] = [0; 4096];
    sdk_init(unsafe { &mut HEAP });

    const KEY: &str = "foo";
    let mut keyval: KeyValueData = [0u8; KEY_VALUE_DATA_SIZE];
    let _ = match sdk_read_key(KEY, &mut keyval) {
        Err(SDKReadKeyFailed) => info!("read(foo) failed as expected"),
        Err(e) => error!("read error {:?}", e),
        Ok(kv) => error!("read returned {:?}", kv),
    };
    keyval
        .split_at_mut(3)
        .0
        .copy_from_slice(String::from("123").as_bytes());
    let _ = match sdk_write_key(KEY, &keyval) {
        Ok(_) => info!("write ok"),
        Err(e) => error!("write error {:?}", e),
    };
    let _ = match sdk_read_key(KEY, &mut keyval) {
        Err(SDKReadKeyFailed) => info!("read(foo) failed as expected"),
        Err(e) => error!("read failed: {:?}", e),
        Ok(kv) => error!("read returned {:?}", kv),
    };
    let _ = match sdk_delete_key(KEY) {
        Ok(_) => info!("delete ok"),
        Err(e) => error!("delete error {:?}", e),
    };
    let _ = match sdk_delete_key(KEY) {
        Ok(_) => info!("delete ok (for missing key)"),
        Err(e) => error!("delete error {:?}", e),
    };
}
