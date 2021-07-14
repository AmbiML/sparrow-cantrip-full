//! Cantrip OS command line interface

// This brief bootstrap of Rust-in-Cantrip prototypes a minimal modular design
// for the DebugConsole CLI use case.
//
// * cantrip_io Read/Write interface (or move to std::, but that requires alloc)
// * cantrip_uart_client implementation of the cantrip_io interface
// * cantrip_line_reader
// * cantrip_shell
// * cantrip_debug_console main entry point fn run()

// std:: requires at least an allocator, which Cantrip does not have yet. For now
// the CLI will be implemented with only core::.
#![no_std]
// NB: not really what we want but resolves undefineds for now
#![feature(default_alloc_error_handler)]

extern crate panic_halt;

use cantrip_shell;
use cantrip_uart_client;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static CANTRIP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[no_mangle]
// NB: use post_init insted of pre_init so syslog interface is setup
pub extern "C" fn post_init() {
    // TODO(sleffler): temp until we integrate with seL4
    static mut HEAP_MEMORY: [u8; 16 * 1024] = [0; 16 * 1024];
    unsafe {
        CANTRIP_ALLOCATOR
            .lock()
            .init(HEAP_MEMORY.as_mut_ptr() as usize, HEAP_MEMORY.len());
    }
}

/// Entry point for DebugConsole. Runs the shell with UART IO.
#[no_mangle]
pub extern "C" fn run() -> ! {
    let mut tx = cantrip_uart_client::Tx {};
    let mut rx = cantrip_uart_client::Rx {};
    cantrip_shell::repl(&mut tx, &mut rx);
}
