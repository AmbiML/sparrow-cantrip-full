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

    let _ = sdk_log(&format!(
        "sdk_timer_cancel returned {:?} with nothing running",
        sdk_timer_cancel(0)
    ));
    let _ = sdk_log(&format!(
        "sdk_timer_poll returned {:?} with nothing running",
        sdk_timer_poll()
    ));
    let _ = sdk_log(&format!(
        "sdk_timer_oneshot returned {:?} with an invalid timer id",
        sdk_timer_oneshot(99, 100) // XXX need TIMERS_PER_CLIENT exported
    ));

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

    const DURATION: TimerDuration = 75; // ms
    if let Err(e) = sdk_timer_periodic(1, DURATION) {
        let _ = sdk_log(&format!("sdk_timer_periodic failed: {:?}", e));
    } else {
        let _ = sdk_log("Timer 1 started");
        let mut ms: TimerDuration = 0;
        for _ in 0..10 {
            let mask = sdk_timer_wait().unwrap();
            if (mask & (1 << 1)) != 0 {
                ms += DURATION;
            }
            let _ = sdk_log(&format!("Timer completed: mask {:#06b} ms {}", mask, ms));
        }
        if let Err(e) = sdk_timer_cancel(1) {
            let _ = sdk_log(&format!("sdk_timer_cancel failed: {:?}", e));
        }
        let _ = sdk_log("Timer 1 canceld");
    }

    if let Err(e) = sdk_timer_periodic(1, DURATION) {
        let _ = sdk_log(&format!("sdk_timer_periodic 1 failed: {:?}", e));
        return;
    }
    let _ = sdk_log("Timer 1 started");
    if let Err(e) = sdk_timer_periodic(2, 2 * DURATION) {
        let _ = sdk_log(&format!("sdk_timer_periodic 2 failed: {:?}", e));
        return;
    }
    let _ = sdk_log("Timer 2 started");

    let mut expire_1 = 0;
    let mut expire_2 = 0;
    for _ in 0..21 {
        let mask = sdk_timer_wait().unwrap();
        if (mask & (1 << 1)) != 0 {
            expire_1 += 1;
        }
        if (mask & (1 << 2)) != 0 {
            expire_2 += 1;
        }
        let _ = sdk_log(&format!(
            "Timer completed: mask {:#06b} 1 {:#2} 2 {:#2}",
            mask, expire_1, expire_2
        ));
    }
    if let Err(e) = sdk_timer_cancel(2) {
        let _ = sdk_log(&format!("sdk_timer_cancel 2 failed: {:?}", e));
    }
    let _ = sdk_log("Timer 2 canceld");
    if let Err(e) = sdk_timer_cancel(1) {
        let _ = sdk_log(&format!("sdk_timer_cancel 1 failed: {:?}", e));
    }
    let _ = sdk_log("Timer 1 canceld");
    let _ = sdk_log("DONE!");
}
