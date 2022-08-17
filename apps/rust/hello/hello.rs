/*
 * Copyright 2021, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#![no_std]
#![no_main]
#![feature(asm)]
#![feature(thread_local)]

// TODO(sleffler): plumb logger to SDKRuntime to eliminate seL4_DebugPutChar
//   (or provide a logger alternative)

use cantrip_os_common::logger::CantripLogger;
use cantrip_os_common::sel4_sys;

use sel4_sys::seL4_IPCBuffer;

const PAGE_SIZE: usize = 4096;

#[no_mangle]
#[thread_local]
static mut __sel4_ipc_buffer: *mut seL4_IPCBuffer = 0 as _;

#[repr(align(4096))]
#[allow(dead_code)]
struct PageAlign {
    data: [u8; PAGE_SIZE],
}
static mut STATIC_TLS: PageAlign = PageAlign {
    data: [0u8; PAGE_SIZE],
};

#[no_mangle]
pub fn _start() {
    unsafe {
        asm!("
        .option push
        .option norelax
        la gp, __global_pointer$
        la tp, {tls}
        lui t1,0
        add t1,t1,tp
        sw a0,0(t1) # __sel4_ipc_buffer>
        addi sp,sp,-16
        sw a0, 12(sp)
        sw a1, 8(sp)
        sw a2, 4(sp)
        sw a3, 0(sp)
        .option pop
        j main",
            tls = sym STATIC_TLS,
            options(noreturn),
        )
    };
}

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
pub fn main(a0: u32, a1: u32, a2: u32, a3: u32) {
    // Setup logger; (XXX belongs in the SDKRuntime)
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    // XXX maybe setup a heap (XXX belongs in the SDKRuntime)

    log::info!("I am a Rust app, hear me roar!");
    log::info!("a0 {:x} a1 {:x} a2 {:x} a3 {:x}", a0, a1, a2, a3);
    log::info!("__sel4_ipc_buffer {:p}", unsafe { __sel4_ipc_buffer });
    log::info!("Done, wimper ...");
}
