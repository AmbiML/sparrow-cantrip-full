/*
 * Copyright 2021, Google LLC
 *
 * Demo to show that concurrent applications can be running.
 *
 * This program prints the first LOG_FIBONACCI_LIMIT Fibonacci numbers
 * to the console, waiting for INTERRUPTS_PER_WAIT interrupts between each
 * number.
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#![no_std]
#![no_main]
#![feature(asm)]

extern crate alloc;
extern crate libcantrip;
use alloc::format;
use cantrip_os_common::allocator;
use sdk_interface::sdk_log;

// How many Fibonacci numbers to write to the log.
const LOG_FIBONACCI_LIMIT: u64 = 80;

const CONFIG_TIMER_TICK_MS: usize = 5;
const INTERRUPTS_PER_VIRT_SEC: u64 = (1000 / CONFIG_TIMER_TICK_MS) as u64;
const INTERRUPTS_PER_WAIT: u64 = 1 * INTERRUPTS_PER_VIRT_SEC;

type ICount = u64;

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

    pub fn log(&self, interrupt_count: ICount) {
        let _ = sdk_log(&format!(
            "n == {}; f == {:x}; interrupt_count == {}; rdtime == {}; virt_sec ~= {:2}",
            self.n,
            self.f1,
            interrupt_count,
            rdtime(),
            virtual_seconds(interrupt_count),
        ));
    }
}

fn wait(interrupt_count_to_wait: ICount, count: &mut ICount) {
    for _ in 0..interrupt_count_to_wait {
        unsafe { asm!("wfi") }
        (*count) += 1;
    }
}

fn virtual_seconds(interrupt_count: ICount) -> f32 {
    (interrupt_count as f32) / (INTERRUPTS_PER_VIRT_SEC as f32)
}

#[allow(unused_assignments)]
fn rdtime() -> u64 {
    let mut upper: u32 = 0;
    let mut lower: u32 = 0;
    let mut upper_reread: u32 = 0;
    loop {
        unsafe {
            asm!(
                "rdtimeh {upper}
        rdtime  {lower}
        rdtimeh {upper_reread}",
                upper = out (reg) upper,
                lower = out (reg) lower,
                upper_reread = out (reg) upper_reread,
            )
        }
        if upper_reread == upper {
            break;
        }
    }
    ((upper as u64) << 32) | (lower as u64)
}

#[no_mangle]
pub fn main() {
    static mut HEAP: [u8; 4096] = [0; 4096];
    unsafe {
        allocator::ALLOCATOR.init(HEAP.as_mut_ptr() as _, HEAP.len());
    }

    let _ = sdk_log("Fibonacci");
    let mut interrupt_count: ICount = 0;
    let mut fib = Fibonacci::new();
    loop {
        wait(INTERRUPTS_PER_WAIT, &mut interrupt_count);
        if fib.n >= LOG_FIBONACCI_LIMIT {
            fib.reset();
        }
        fib.log(interrupt_count);
        fib.increment();
    }
}
