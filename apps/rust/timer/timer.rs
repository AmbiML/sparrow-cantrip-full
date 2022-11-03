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

    let _ = match sdk_timer_oneshot(0, 100) {
        Ok(_) => {
            let _ = sdk_log("Timer 0 started");
            match sdk_timer_wait() {
                Ok(_) => sdk_log(&format!("Timer 0 completed")),
                Err(e) => sdk_log(&format!("sdk_timer_wait failed: {:?}", e)),
            }
        }
        Err(e) => sdk_log(&format!("sdk_timer_oneshot failed: {:?}", e)),
    };

    let _ = sdk_log(&format!(
        "sdk_timer_cancel returned {:?} with nothing running",
        sdk_timer_cancel(0)
    ));
    // XXX sdk_timer_wait blocks, it should return immediately
    //    let _ = sdk_log(&format!("sdk_timer_wait returned {:?} with nothing running", sdk_timer_wait()));

    const DURATION: TimerDuration = 75; // ms
    if let Err(e) = sdk_timer_periodic(1, DURATION) {
        let _ = sdk_log(&format!("sdk_timer_periodic failed: {:?}", e));
    } else {
        let _ = sdk_log("Timer 1 started");
        let mut ms: TimerDuration = 0;
        for _ in 0..10 {
            let _ = sdk_timer_wait();
            ms += DURATION;
            let _ = sdk_log(&format!("Timer 1 completed: {}", ms));
        }
        if let Err(e) = sdk_timer_cancel(1) {
            let _ = sdk_log(&format!("sdk_timer_cancel failed: {:?}", e));
        }
    }

    if let Err(e) = sdk_timer_periodic(1, DURATION) {
        let _ = sdk_log(&format!("sdk_timer_periodic 1 failed: {:?}", e));
        return;
    }
    if let Err(e) = sdk_timer_periodic(2, 2 * DURATION) {
        let _ = sdk_log(&format!("sdk_timer_periodic 2 failed: {:?}", e));
        return;
    }
    let mut ms = 0;
    for _ in 0..20 {
        let _ = sdk_timer_wait();
        ms += DURATION; // XXX don't  know which timer expired
        let _ = sdk_log(&format!("Timer completed: {}", ms));
    }
    if let Err(e) = sdk_timer_cancel(2) {
        let _ = sdk_log(&format!("sdk_timer_cancel 2 failed: {:?}", e));
    }
    if let Err(e) = sdk_timer_cancel(1) {
        let _ = sdk_log(&format!("sdk_timer_cancel 1 failed: {:?}", e));
    }
    let _ = sdk_log("DONE!");
}
