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

use core::slice;
use cantrip_os_common::camkes::Camkes;
use log::LevelFilter;

// NB: this controls filtering log messages from all components because
//   they are setup to send all log messges to the console.
#[cfg(feature = "LOG_DEBUG")]
const INIT_LOG_LEVEL: LevelFilter = LevelFilter::Debug;
#[cfg(feature = "LOG_TRACE")]
const INIT_LOG_LEVEL: LevelFilter = LevelFilter::Trace;
#[cfg(not(any(feature = "LOG_DEBUG", feature = "LOG_TRACE")))]
const INIT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

extern "C" {
    static cpio_archive: *const u8; // CPIO archive of built-in files
}

static mut CAMKES: Camkes = Camkes::new("DebugConsole");

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    const HEAP_SIZE: usize = 16 * 1024;
    static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    CAMKES.pre_init(INIT_LOG_LEVEL, &mut HEAP_MEMORY);
}

/// Entry point for DebugConsole. Runs the shell with UART IO.
#[no_mangle]
pub extern "C" fn run() -> ! {
    let mut tx = cantrip_uart_client::Tx::new();
    let mut rx = cantrip_io::BufReader::new(cantrip_uart_client::Rx::new());
    let cpio_archive_ref = unsafe {
        // XXX want begin-end or begin+size instead of a fixed-size block
        slice::from_raw_parts(cpio_archive, 16777216)
    };
    cantrip_shell::repl(&mut tx, &mut rx, cpio_archive_ref);
}
