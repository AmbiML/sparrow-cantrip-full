/*
 * Copyright 2021, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#![no_std]
#![no_main]

extern crate alloc;
extern crate libcantrip;
use alloc::format;
use alloc::string::String;
use cantrip_os_common::allocator;
use cantrip_os_common::logger::CantripLogger;
use cantrip_os_common::sel4_sys;
use sdk_interface::*;
use SDKRuntimeError::*;

// Message output is sent through the cantrip-os-logger which calls logger_log
// to deliver data to the console. We use seL4_DebugPutChar to write to the
// console which only works if DEBUG_PRINTING is enabled in the kernel.
#[no_mangle]
#[allow(unused_variables)]
pub fn logger_log(_level: u8, msg: *const cstr_core::c_char) {
    #[cfg(feature = "CONFIG_PRINTING")]
    unsafe {
        for c in cstr_core::CStr::from_ptr(msg).to_bytes() {
            let _ = sel4_sys::seL4_DebugPutChar(*c);
        }
        let _ = sel4_sys::seL4_DebugPutChar(b'\n');
    }
}

#[no_mangle]
// XXX need SDK specification of main, use hack for now
pub fn main() {
    // Setup logger; (XXX maybe belongs in the SDKRuntime)
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    static mut HEAP: [u8; 4096] = [0; 4096];
    unsafe {
        allocator::ALLOCATOR.init(HEAP.as_mut_ptr() as _, HEAP.len());
    }

    const KEY: &str = "foo";
    let mut keyval: KeyValueData = [0u8; KEY_VALUE_DATA_SIZE];
    let _ = match sdk_read_key(KEY, &mut keyval) {
        Err(SDKReadKeyFailed) => sdk_log("read(foo) failed as expected"),
        Err(e) => sdk_log(&format!("read error {:?}", e)),
        Ok(kv) => sdk_log(&format!("read returned {:?}", kv)),
    };
    keyval
        .split_at_mut(3)
        .0
        .copy_from_slice(String::from("123").as_bytes());
    let _ = match sdk_write_key(KEY, &keyval) {
        Ok(_) => sdk_log("write ok"),
        Err(e) => sdk_log(&format!("write error {:?}", e)),
    };
    let _ = match sdk_read_key(KEY, &mut keyval) {
        Err(SDKReadKeyFailed) => sdk_log("read(foo) failed as expected"),
        Err(e) => sdk_log(&format!("read failed: {:?}", e)),
        Ok(kv) => sdk_log(&format!("read returned {:?}", kv)),
    };
    let _ = match sdk_delete_key(KEY) {
        Ok(_) => sdk_log("delete ok"),
        Err(e) => sdk_log(&format!("delete error {:?}", e)),
    };
    let _ = match sdk_delete_key(KEY) {
        Ok(_) => sdk_log("delete ok (for missing key)"),
        Err(e) => sdk_log(&format!("delete error {:?}", e)),
    };
}
