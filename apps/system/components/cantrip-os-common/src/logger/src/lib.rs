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

#![cfg_attr(not(test), no_std)]
//error[E0658]: dereferencing raw mutable pointers in statics is unstable
#![feature(const_mut_refs)]

use core::str::from_utf8_unchecked;
use core2::io::{Cursor, Write};
use log::{Metadata, Record};
use num_enum::{FromPrimitive, IntoPrimitive};
pub use paste::*; // re-export for macros
use serde::{Deserialize, Serialize};
use spin::Mutex;

pub const MAX_MSG_LEN: usize = 2048;

#[repr(usize)]
#[derive(Debug, Default, Eq, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum LoggerError {
    Success = 0,
    SerializeFailed,
    DeserializeFailed,
    // Generic errors.
    LogFailed,
    #[default]
    UnknownError,
}
impl From<LoggerError> for Result<(), LoggerError> {
    fn from(err: LoggerError) -> Result<(), LoggerError> {
        if err == LoggerError::Success {
            Ok(())
        } else {
            Err(err)
        }
    }
}

#[macro_export]
macro_rules! static_logger {
    ($inf_tag:ident) => {
        static_logger!($inf_tag, log::LevelFilter::Trace);
    };
    ($inf_tag:ident, $inf_level:expr) => {
        $crate::paste! {
            static CANTRIP_LOGGER: CantripLogger = logger::CantripLogger::new(
                [<$inf_tag:upper _INTERFACE_ENDPOINT>],
                unsafe { &mut [<$inf_tag:upper _INTERFACE_DATA>].data },
            );
            log::set_logger(&CANTRIP_LOGGER).unwrap();
            log::set_max_level($inf_level);
        }
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LoggerRequest<'a> {
    Log { level: u8, msg: &'a str },
}

pub struct CantripLogger {
    endpoint: sel4_sys::seL4_CPtr,
    buffer: Mutex<&'static mut [u8]>,
}
impl CantripLogger {
    pub const fn new(endpoint: sel4_sys::seL4_CPtr, buffer: &'static mut [u8]) -> Self {
        Self {
            endpoint,
            buffer: Mutex::new(buffer),
        }
    }
}
impl log::Log for CantripLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool { true }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // TODO(sleffler): split self.buffer instead of using the stack
            let mut buf = [0u8; MAX_MSG_LEN];
            let mut cur = Cursor::new(&mut buf[..]);
            // Log msgs are of the form: '<target>::<fmt'd-msg>
            write!(&mut cur, "{}::{}", record.target(), record.args()).unwrap_or_else(|_| {
                // Too big, indicate overflow with a trailing "...".
                cur.set_position((MAX_MSG_LEN - 3) as u64);
                cur.write(b"...").expect("write!");
            });
            // NB: this releases the ref on buf held by the Cursor
            let pos = cur.position() as usize;

            let _ = postcard::to_slice(
                &LoggerRequest::Log {
                    level: record.level() as u8,
                    msg: unsafe { from_utf8_unchecked(&buf[..pos]) },
                    // XXX Ok a hack
                },
                *self.buffer.lock(),
            )
            .map(|_| unsafe {
                sel4_sys::seL4_Call(
                    self.endpoint,
                    sel4_sys::seL4_MessageInfo::new(
                        /*label=*/ 0, /*capsUnwrapped=*/ 0, /*extraCaps=*/ 0,
                        /*length=*/ 0,
                    ),
                )
                .get_label()
            });
        }
    }

    fn flush(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrayvec::ArrayVec;
    use log::{debug, error, info, trace, warn};

    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    static mut MSGS: ArrayVec<[u8; MAX_MSG_LEN], 8> = ArrayVec::new_const();

    // Resets msg collection; used at the start of each test.
    fn reset_msgs() {
        unsafe {
            MSGS.clear();
        }
    }

    #[no_mangle]
    pub extern "C" fn logger_log(_level: u8, msg: *const cstr_core::c_char) {
        unsafe {
            // NB: this depends on msg pointing to the caller's array
            MSGS.push(*(msg as *const [u8; MAX_MSG_LEN]));
        }
    }

    // Formats a log message as done by CantripLogger and returns a zero-padded
    // Vec holding the message.
    fn log_msg(_level: log::Level, msg: &str) -> Result<Vec<u8>, std::io::Error> {
        let mut v = Vec::new();
        use std::io::Write;
        write!(&mut v, "{}::{}\0", "logger::tests", msg)?;
        v.resize(MAX_MSG_LEN, 0);
        assert_eq!(v.len(), MAX_MSG_LEN);
        Ok(v)
    }

    fn pop_and_expect_none(level: log::Level) {
        unsafe {
            let msg = MSGS.pop();
            assert_eq!(
                msg, None,
                "Assertioon failed for level {}: expected None, got {:?}",
                level, msg
            );
        }
    }

    fn pop_and_check_result(level: log::Level, test_msg: &str) {
        unsafe {
            let msg = MSGS.pop();
            assert!(
                msg != None,
                "Assertion failed for level {}: no log msg collected",
                level
            );
            let expected = log_msg(level, test_msg).unwrap();
            let observed = msg.unwrap().to_vec();
            assert_eq!(
                expected,
                observed,
                "Assertion failed for level {}: expected {}, got {}",
                level,
                String::from_utf8_lossy(expected.as_slice()),
                String::from_utf8_lossy(observed.as_slice())
            );
        }
    }

    // NB: to run these sequentially use --test-threads=1; otherwise
    // cargo will use multiple threads and you will get failures from
    // multiple users of MSGS and the global logger; e.g.
    //     cargo test -- --test-threads=1

    #[test]
    fn test_each_log_level_works() {
        reset_msgs();

        let _ = log::set_logger(&CANTRIP_LOGGER);
        log::set_max_level(log::LevelFilter::Trace);

        let debug_msg = "hello debug";
        debug!("{}", debug_msg);
        pop_and_check_result(log::Level::Debug, debug_msg);

        let info_msg = "hello info";
        info!("{}", info_msg);
        pop_and_check_result(log::Level::Info, info_msg);

        let warn_msg = "hello warn";
        warn!("{}", warn_msg);
        pop_and_check_result(log::Level::Warn, warn_msg);

        let error_msg = "hello error";
        error!("{}", error_msg);
        pop_and_check_result(log::Level::Error, error_msg);

        let trace_msg = "hello trace";
        trace!("{}", trace_msg);
        pop_and_check_result(log::Level::Trace, trace_msg);
    }

    #[test]
    fn test_max_log_level() {
        reset_msgs();

        let _ = log::set_logger(&CANTRIP_LOGGER);
        // With filtering at Debug level, levels below should come through.
        log::set_max_level(log::LevelFilter::Debug);

        let debug_msg = "hello debug";
        debug!("{}", debug_msg);
        pop_and_check_result(log::Level::Debug, debug_msg);

        let info_msg = "hello info";
        info!("{}", info_msg);
        pop_and_check_result(log::Level::Info, info_msg);

        let warn_msg = "hello warn";
        warn!("{}", warn_msg);
        pop_and_check_result(log::Level::Warn, warn_msg);

        let error_msg = "hello error";
        error!("{}", error_msg);
        pop_and_check_result(log::Level::Error, error_msg);

        let trace_msg = "hello trace";
        trace!("{}", trace_msg);
        pop_and_expect_none(log::Level::Trace);

        // Dynamically adjust the log level
        log::set_max_level(log::LevelFilter::Info);

        info!("{}", info_msg);
        pop_and_check_result(log::Level::Info, info_msg);

        debug!("{}", debug_msg);
        pop_and_expect_none(log::Level::Debug);
    }

    #[test]
    fn test_formatting() {
        reset_msgs();

        let _ = log::set_logger(&CANTRIP_LOGGER);
        log::set_max_level(log::LevelFilter::Debug);

        debug!("a {} b {} c {} d {:#x}", 99, "foo", 3.4, 32);
        pop_and_check_result(
            log::Level::Debug,
            &format!("a {} b {} c {} d {:#x}", 99, "foo", 3.4, 32)[..],
        );
    }

    #[test]
    fn test_too_long() {
        reset_msgs();

        let _ = log::set_logger(&CANTRIP_LOGGER);
        log::set_max_level(log::LevelFilter::Debug);

        // Guarantee formatted message is > MAX_MSG_LEN
        let mut debug_msg = "debug".repeat((MAX_MSG_LEN / 5) + 1);
        debug!("{}", debug_msg);

        // Blech, must take into account log msg formatting.
        debug_msg.truncate(MAX_MSG_LEN - 4 - "logger::tests::".len());
        debug_msg.push_str("...");
        pop_and_check_result(log::Level::Debug, &debug_msg[..]);
    }

    #[test]
    fn test_embedded_nul() {
        reset_msgs();

        let _ = log::set_logger(&CANTRIP_LOGGER);
        log::set_max_level(log::LevelFilter::Debug);

        let debug_msg = "bar\0foo";
        debug!("{}", debug_msg);
        pop_and_check_result(log::Level::Debug, "<embedded nul>");
    }
}
