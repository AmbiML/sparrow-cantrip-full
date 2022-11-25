/*
 * Copyright 2021, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#![no_std]
#![no_main]

use libcantrip::sdk_init;
use log::{error, info};
use sdk_interface::*;

#[no_mangle]
pub fn main() {
    static mut HEAP: [u8; 4096] = [0; 4096];
    sdk_init(unsafe { &mut HEAP });

    info!(
        "sdk_timer_cancel returned {:?} with nothing running",
        sdk_timer_cancel(0)
    );
    info!("sdk_timer_poll returned {:?} with nothing running", sdk_timer_poll());
    info!(
        "sdk_timer_oneshot returned {:?} with an invalid timer id",
        sdk_timer_oneshot(99, 100)
    ); // XXX need TIMERS_PER_CLIENT exported;

    let _ = match sdk_timer_oneshot(0, 100) {
        Ok(_) => {
            info!("Timer 0 started");
            match sdk_timer_wait() {
                Ok(_) => info!("Timer 0 completed"),
                Err(e) => error!("sdk_timer_wait failed: {:?}", e),
            }
        }
        Err(e) => error!("sdk_timer_oneshot failed: {:?}", e),
    };

    const DURATION: TimerDuration = 75; // ms
    if let Err(e) = sdk_timer_periodic(1, DURATION) {
        error!("sdk_timer_periodic failed: {:?}", e);
    } else {
        info!("Timer 1 started");
        let mut ms: TimerDuration = 0;
        for _ in 0..10 {
            let mask = sdk_timer_wait().unwrap();
            if (mask & (1 << 1)) != 0 {
                ms += DURATION;
            }
            let _ = info!("Timer completed: mask {:#06b} ms {}", mask, ms);
        }
        if let Err(e) = sdk_timer_cancel(1) {
            error!("sdk_timer_cancel failed: {:?}", e);
        }
        let _ = info!("Timer 1 canceld");
    }

    if let Err(e) = sdk_timer_periodic(1, DURATION) {
        error!("sdk_timer_periodic 1 failed: {:?}", e);
        return;
    }
    let _ = sdk_log("Timer 1 started");
    if let Err(e) = sdk_timer_periodic(2, 2 * DURATION) {
        error!("sdk_timer_periodic 2 failed: {:?}", e);
        return;
    }
    info!("Timer 2 started");

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
        info!(
            "Timer completed: mask {:#06b} 1 {:#2} 2 {:#2}",
            mask, expire_1, expire_2
        );
    }
    if let Err(e) = sdk_timer_cancel(2) {
        error!("sdk_timer_cancel 2 failed: {:?}", e);
    }
    info!("Timer 2 canceld");
    if let Err(e) = sdk_timer_cancel(1) {
        error!("sdk_timer_cancel 1 failed: {:?}", e);
    }
    info!("Timer 1 canceld");
    info!("DONE!");
}
