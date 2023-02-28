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

use cantrip_os_common::camkes::Camkes;
use core::fmt::Write;
use cstr_core::CStr;
use log::LevelFilter;

use cantrip_io as io;

// NB: this controls filtering log messages from all components because
//   they are setup to send all log messges to the console.
#[cfg(feature = "LOG_DEBUG")]
const INIT_LOG_LEVEL: LevelFilter = LevelFilter::Debug;
#[cfg(feature = "LOG_TRACE")]
const INIT_LOG_LEVEL: LevelFilter = LevelFilter::Trace;
#[cfg(not(any(feature = "LOG_DEBUG", feature = "LOG_TRACE")))]
const INIT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

static mut CAMKES: Camkes = Camkes::new("DebugConsole");

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    const HEAP_SIZE: usize = 12 * 1024;
    static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    CAMKES.pre_init(INIT_LOG_LEVEL, &mut HEAP_MEMORY);
}

/// Tx io trait that uses the kernel if console output is
/// is supported, otherwise discards all writes.
struct Tx {}
impl Tx {
    pub fn new() -> Self { Self {} }
}
impl Default for Tx {
    fn default() -> Self { Self::new() }
}
impl io::Write for Tx {
    #[cfg(not(feature = "CONFIG_PRINTING"))]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { Ok(buf.len() as usize) }
    #[cfg(feature = "CONFIG_PRINTING")]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &b in buf {
            unsafe {
                cantrip_os_common::sel4_sys::seL4_DebugPutChar(b);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

// Returns a trait-compatible Tx based on the selected features.
// NB: must use "return expr;" to avoid confusing the compiler.
fn get_tx() -> impl io::Write {
    #[cfg(feature = "CONFIG_PLAT_SPARROW")]
    return cantrip_uart_client::Tx::new();

    #[cfg(not(feature = "CONFIG_PLAT_SPARROW"))]
    return Tx::new();
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
        let output: &mut dyn io::Write = &mut get_tx();
        let _ = writeln!(output, "{}", CStr::from_ptr(msg).to_str().unwrap());
    }
}

// Run any "autostart.repl" file in the eFLASH through the shell with output
// sent either to the console or /dev/null depending on the feature selection.
#[cfg(feature = "autostart_support")]
fn run_autostart_shell() {
    // Rx data comes from the embedded script
    // Tx data goes to either the uart or /dev/null
    // XXX test if autostart.repl is present
    let mut rx = cantrip_io::BufReader::new("source -q autostart.repl\n".as_bytes());
    cantrip_shell::repl_eof(&mut get_tx(), &mut rx);
}

// Runs an interactive shell using the Sparrow UART.
#[cfg(feature = "CONFIG_PLAT_SPARROW")]
fn run_sparrow_shell() -> ! {
    let mut tx = cantrip_uart_client::Tx::new();
    let mut rx = io::BufReader::new(cantrip_uart_client::Rx::new());
    cantrip_shell::repl(&mut tx, &mut rx);
}

/// Entry point for DebugConsole. Optionally runs an autostart script
/// after which it runs an interactive shell with UART IO.
#[no_mangle]
pub extern "C" fn run() {
    #[cfg(feature = "autostart_support")]
    run_autostart_shell();

    #[cfg(feature = "CONFIG_PLAT_SPARROW")]
    run_sparrow_shell();
}
