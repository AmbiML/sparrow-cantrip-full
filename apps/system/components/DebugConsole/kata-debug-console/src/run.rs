//! Cantrip OS command line interface

// This brief bootstrap of Rust-in-Cantrip prototypes a minimal modular design
// for the DebugConsole CLI use case.
//
// * cantrip_io Read/Write interface (or move to std::, but that requires alloc)
// * cantrip_uart_client implementation of the cantrip_io interface
// * cantrip_line_reader
// * cantrip_shell
// * cantrip_debug_console main entry point fn run()

#![no_std]

use cantrip_io;
use cantrip_os_common::allocator;
use cantrip_os_common::logger::CantripLogger;
use cantrip_os_common::sel4_sys;
use cantrip_os_common::slot_allocator;
use cantrip_shell;
use cantrip_uart_client;
use log::trace;

use sel4_sys::seL4_CPtr;

use slot_allocator::CANTRIP_CSPACE_SLOTS;

extern "C" {
    static SELF_CNODE_FIRST_SLOT: seL4_CPtr;
    static SELF_CNODE_LAST_SLOT: seL4_CPtr;
}

#[no_mangle]
pub extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    // NB: set to Trace for early-boot msgs
    log::set_max_level(log::LevelFilter::Debug);

    // TODO(b/200946906): Review per-component heap allocations, including this one.
    const HEAP_SIZE: usize = 1 << 20;
    static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    unsafe {
        allocator::ALLOCATOR.init(HEAP_MEMORY.as_mut_ptr() as usize, HEAP_MEMORY.len());
        trace!(
            "setup heap: start_addr {:p} size {}",
            HEAP_MEMORY.as_ptr(),
            HEAP_MEMORY.len()
        );
    }

    unsafe {
        CANTRIP_CSPACE_SLOTS.init(
            /*first_slot=*/ SELF_CNODE_FIRST_SLOT,
            /*size=*/ SELF_CNODE_LAST_SLOT - SELF_CNODE_FIRST_SLOT
        );
        trace!("setup cspace slots: first slot {} free {}",
               CANTRIP_CSPACE_SLOTS.base_slot(),
               CANTRIP_CSPACE_SLOTS.free_slots());
    }
}

/// Entry point for DebugConsole. Runs the shell with UART IO.
#[no_mangle]
pub extern "C" fn run() -> ! {
    trace!("run");
    let mut tx = cantrip_uart_client::Tx::new();
    let mut rx = cantrip_io::BufReader::new(cantrip_uart_client::Rx::new());
    cantrip_shell::repl(&mut tx, &mut rx);
}
