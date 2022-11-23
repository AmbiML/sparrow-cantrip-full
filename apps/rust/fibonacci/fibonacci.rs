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

extern crate alloc;
extern crate libcantrip;
use alloc::format;
use cantrip_os_common::allocator;
use cantrip_os_common::logger::CantripLogger;
use sdk_interface::sdk_log;

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
        let _ = sdk_log(&format!("[{:2}] {:20}  {}", self.n, self.f1, time_ms));
    }
}

// Connect the logger so panic msgs are displayed.
#[no_mangle]
pub fn logger_log(_level: u8, msg: *const cstr_core::c_char) {
    let _ = sdk_log(unsafe { cstr_core::CStr::from_ptr(msg).to_str().unwrap() });
}

#[no_mangle]
pub fn main() {
    // NB: setup for panic messages to be logged
    static mut HEAP: [u8; 4096] = [0; 4096];
    unsafe {
        allocator::ALLOCATOR.init(HEAP.as_mut_ptr() as _, HEAP.len());
    }
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();

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
