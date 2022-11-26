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
use cantrip_os_common::allocator;
use cantrip_os_common::logger::CantripLogger;
use cantrip_os_common::sel4_sys;
use sdk_interface::*;

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
pub fn main() {
    // Setup logger for panic; (XXX maybe belongs in the SDKRuntime)
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    static mut HEAP: [u8; 4096] = [0; 4096];
    unsafe {
        allocator::ALLOCATOR.init(HEAP.as_mut_ptr() as _, HEAP.len());
    }

    const MODEL_ID: &str = "mobilenet_v1_emitc_static";
    const NONEXISTENT_ID: &str = "nonexistent";
    //    let _ = sdk_log(&format!(
    //        "sdk_model_cancel returned {:?} with nothing running",
    //        sdk_model_cancel(0)
    //    ));
    let _ = sdk_log(&format!(
        "sdk_model_oneshot({}) returned {:?} (as expected)",
        NONEXISTENT_ID,
        sdk_model_oneshot(NONEXISTENT_ID),
    ));

    let _ = match sdk_model_oneshot(MODEL_ID) {
        Ok(id) => {
            let _ = sdk_log(&format!("{} started, id {}", MODEL_ID, id));
            match sdk_model_wait() {
                Ok(_) => sdk_log(&format!("{} completed", MODEL_ID)),
                Err(e) => sdk_log(&format!("sdk_model_wait failed: {:?}", e)),
            }
        }
        Err(e) => sdk_log(&format!("sdk_model_oneshot({}) failed: {:?}", MODEL_ID, e)),
    };

    const DURATION: TimerDuration = 1000; // 1s
    match sdk_model_periodic(MODEL_ID, DURATION) {
        Ok(id) => {
            let _ = sdk_log(&format!("Model {} started, id {}", MODEL_ID, id));
            let mut ms: TimerDuration = 0;
            for _ in 0..10 {
                let mask = sdk_model_wait().unwrap();
                if (mask & (1 << id)) != 0 {
                    ms += DURATION;
                }
                let _ = sdk_log(&format!("Model completed: mask {:#06b} ms {}", mask, ms));
            }
            if let Err(e) = sdk_model_cancel(id) {
                let _ = sdk_log(&format!("sdk_tmodel_cancel failed: {:?}", e));
            }
            let _ = sdk_log(&format!("Model {} canceled", id));
        }
        Err(e) => {
            let _ = sdk_log(&format!("sdk_model_periodic({}) failed: {:?}", MODEL_ID, e));
        }
    }
    let _ = sdk_log("DONE!");
}
