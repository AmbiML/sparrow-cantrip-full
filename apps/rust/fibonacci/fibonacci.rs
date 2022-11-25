/*
 * Copyright 2021, Google LLC
 *
 * Demo to show that concurrent applications can be running.
 *
 * This program prints the first LOG_FIBONACCI_LIMIT Fibonacci numbers
 * to the console, waiting a fixed interval between each number.
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#![no_std]
#![no_main]

use libcantrip::sdk_init;
use log::info;
use sdk_interface::*;

// How many Fibonacci numbers to write to the log.
const LOG_FIBONACCI_LIMIT: u64 = 80;

struct Fibonacci {
    f1: u64,
    f2: u64,
    n: u64,
}
impl Fibonacci {
    pub fn new() -> Self { Self { f1: 0, f2: 1, n: 0 } }
    pub fn increment(&mut self) {
        let swap: u64 = self.f2;
        self.f2 = self.f1 + self.f2;
        self.f1 = swap;
        self.n += 1;
    }
    pub fn reset(&mut self) {
        self.f1 = 0;
        self.f2 = 1;
        self.n = 0;
    }

    pub fn log(&self, time_ms: TimerDuration) {
        info!("[{:2}] {:20}  {}", self.n, self.f1, time_ms);
    }
}

#[no_mangle]
pub fn main() {
    static mut HEAP: [u8; 4096] = [0; 4096];
    sdk_init(unsafe { &mut HEAP });

    let mut fib = Fibonacci::new();
    const INTERVAL: TimerDuration = 100; // 100ms
    let mut time_ms = 0;
    sdk_timer_periodic(0, INTERVAL).expect("periodic");
    loop {
        if fib.n >= LOG_FIBONACCI_LIMIT {
            fib.reset();
        }
        fib.log(time_ms);
        fib.increment();
        sdk_timer_wait().expect("wait");
        time_ms += INTERVAL;
    }
}
