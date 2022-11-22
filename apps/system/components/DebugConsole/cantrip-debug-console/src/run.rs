// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Cantrip OS command line interface
//!
//! This brief bootstrap of Rust-in-Cantrip prototypes a minimal modular design
//! for the DebugConsole CLI use case.
//!
//! * cantrip_io Read/Write interface (or move to std::, but that requires alloc)
//! * cantrip_uart_client implementation of the cantrip_io interface
//! * cantrip_line_reader
//! * cantrip_shell
//! * cantrip_debug_console main entry point fn run()

#![no_std]
#![allow(clippy::missing_safety_doc)]

use core::fmt::Write;
use core::slice;
use cpio::CpioNewcReader;
use cstr_core::CStr;
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

// Returns a trait-compatible Tx based on the selected features.
// NB: must use "return expr;" to avoid confusing the compiler.
fn get_tx() -> impl cantrip_io::Write {
    #[cfg(feature = "CONFIG_PLAT_SPARROW")]
    return cantrip_uart_client::Tx::new();

    #[cfg(not(feature = "CONFIG_PLAT_SPARROW"))]
    return default_uart_client::Tx::new();
}

/// Console logging interface.
#[no_mangle]
pub unsafe extern "C" fn logger_log(level: u8, msg: *const cstr_core::c_char) {
    use log::Level;
    let l = match level {
        x if x == Level::Error as u8 => Level::Error,
        x if x == Level::Warn as u8 => Level::Warn,
        x if x == Level::Info as u8 => Level::Info,
        x if x == Level::Debug as u8 => Level::Debug,
        _ => Level::Trace,
    };
    if l <= log::max_level() {
        // TODO(sleffler): is the uart driver ok w/ multiple writers?
        // TODO(sleffler): fallback to seL4_DebugPutChar?
        let output: &mut dyn cantrip_io::Write = &mut get_tx();
        let _ = writeln!(output, "{}", CStr::from_ptr(msg).to_str().unwrap());
    }
}

// If the builtins archive includes an "autostart.repl" file it is run
// through the shell with output sent either to the console or /dev/null
// depending on the feature selection.
#[cfg(feature = "autostart_support")]
fn run_autostart_shell(cpio_archive_ref: &[u8]) {
    const AUTOSTART_NAME: &str = "autostart.repl";

    let mut autostart_script: Option<&[u8]> = None;
    let reader = CpioNewcReader::new(cpio_archive_ref);
    for e in reader {
        if e.is_err() {
            break;
        }
        let entry = e.unwrap();
        if entry.name == AUTOSTART_NAME {
            autostart_script = Some(entry.data);
            break;
        }
    }
    if let Some(script) = autostart_script {
        // Rx data comes from the embedded script
        // Tx data goes to either the uart or /dev/null
        let mut rx = cantrip_io::BufReader::new(default_uart_client::Rx::new(script));
        cantrip_shell::repl_eof(&mut get_tx(), &mut rx, cpio_archive_ref);
    }
}

// Runs an interactive shell using the Sparrow UART.
#[cfg(feature = "CONFIG_PLAT_SPARROW")]
fn run_sparrow_shell(cpio_archive_ref: &[u8]) -> ! {
    let mut tx = cantrip_uart_client::Tx::new();
    let mut rx = cantrip_io::BufReader::new(cantrip_uart_client::Rx::new());
    cantrip_shell::repl(&mut tx, &mut rx, cpio_archive_ref);
}

/// Entry point for DebugConsole. Optionally runs an autostart script
/// after which it runs an interactive shell with UART IO.
#[no_mangle]
pub extern "C" fn run() {
    let cpio_archive_ref = unsafe {
        // XXX want begin-end or begin+size instead of a fixed-size block
        slice::from_raw_parts(cpio_archive, 16777216)
    };

    #[cfg(feature = "autostart_support")]
    run_autostart_shell(cpio_archive_ref);

    #[cfg(feature = "CONFIG_PLAT_SPARROW")]
    run_sparrow_shell(cpio_archive_ref);
}
