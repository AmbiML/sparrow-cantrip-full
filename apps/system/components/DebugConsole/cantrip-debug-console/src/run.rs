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
//error[E0658]: dereferencing raw mutable pointers in statics is unstable
#![feature(const_mut_refs)]

use cantrip_os_common::camkes;
use cantrip_os_common::logger;
use core::fmt::Write;

use camkes::*;

use log::LevelFilter;
use log::Metadata;
use log::Record;

use logger::LoggerError;
use logger::LoggerRequest;

use cantrip_io as io;

// NB: this controls filtering log messages from all components because
//   they are setup to send all log messges to the console.
#[cfg(feature = "LOG_DEBUG")]
const INIT_LOG_LEVEL: LevelFilter = LevelFilter::Debug;
#[cfg(feature = "LOG_TRACE")]
const INIT_LOG_LEVEL: LevelFilter = LevelFilter::Trace;
#[cfg(not(any(feature = "LOG_DEBUG", feature = "LOG_TRACE")))]
const INIT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

// Generated code...
mod generated {
    include!(concat!(env!("SEL4_OUT_DIR"), "/../debug_console/camkes.rs"));
}
use generated::*;

struct DebugConsoleControlThread;
impl CamkesThreadInterface for DebugConsoleControlThread {
    fn pre_init() {
        const HEAP_SIZE: usize = 12 * 1024;
        static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
        unsafe {
            CAMKES.pre_init(&mut HEAP_MEMORY);
        }

        log::set_logger(&LoggerInterfaceThread).unwrap();
        log::set_max_level(INIT_LOG_LEVEL);
    }

    // Entry point for DebugConsole. Optionally runs an autostart script
    // after which it runs an interactive shell with UART IO.
    fn run() {
        #[cfg(feature = "autostart_support")]
        run_autostart_shell();

        #[cfg(feature = "interactive_shell")]
        run_sparrow_shell();
    }
}

/// Console logging interface.
struct LoggerInterfaceThread;
impl CamkesThreadInterface for LoggerInterfaceThread {
    fn run() {
        rpc_shared_recv!(logger, logger::MAX_MSG_LEN, LoggerError::Success);
    }
}
impl LoggerInterfaceThread {
    fn dispatch(
        _client_badge: usize,
        request_buffer: &[u8],
        _reply_buffer: &mut [u8],
    ) -> Result<(), LoggerError> {
        let request = match postcard::from_bytes::<LoggerRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(LoggerError::DeserializeFailed),
        };
        match request {
            LoggerRequest::Log { level, msg } => Self::log_request(level, msg),
        }
    }
    fn log_request(level: u8, msg: &str) -> Result<(), LoggerError> {
        use log::Level;
        let l = match level {
            x if x == Level::Error as u8 => Level::Error,
            x if x == Level::Warn as u8 => Level::Warn,
            x if x == Level::Info as u8 => Level::Info,
            x if x == Level::Debug as u8 => Level::Debug,
            _ => Level::Trace,
        };
        if l <= log::max_level() {
            Self::log_msg(msg);
        }
        Ok(())
    }
    fn log_msg(msg: &str) {
        // TODO(sleffler): safeguard multiple writers
        let output: &mut dyn io::Write = &mut get_tx();
        let _ = writeln!(output, "{}", msg);
    }
}
// Scaffolding for DebugConsole log msgs.
impl log::Log for LoggerInterfaceThread {
    fn enabled(&self, metadata: &Metadata) -> bool { metadata.level() <= log::max_level() }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // NB: stack is too small  for 2K but this should really be in self
            const MAX_MSG_LEN: usize = 1024;
            use core2::io::{Cursor, Write};
            let mut buf = [0u8; MAX_MSG_LEN];
            let mut cur = Cursor::new(&mut buf[..]);
            write!(&mut cur, "{}::{}", record.target(), record.args()).unwrap_or_else(|_| {
                // Too big, indicate overflow with a trailing "...".
                cur.set_position((MAX_MSG_LEN - 3) as u64);
                cur.write(b"...").expect("write!");
            });
            // NB: this releases the ref on buf held by the Cursor
            let pos = cur.position() as usize;
            Self::log_msg(core::str::from_utf8(&buf[..pos]).unwrap());
        }
    }
    fn flush(&self) {}
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
    #[cfg(feature = "interactive_shell")]
    return cantrip_uart_client::Tx::new();

    #[cfg(not(feature = "interactive_shell"))]
    return Tx::new();
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
#[cfg(feature = "interactive_shell")]
fn run_sparrow_shell() -> ! {
    let mut tx = cantrip_uart_client::Tx::new();
    let mut rx = io::BufReader::new(cantrip_uart_client::Rx::new());
    cantrip_shell::repl(&mut tx, &mut rx);
}
