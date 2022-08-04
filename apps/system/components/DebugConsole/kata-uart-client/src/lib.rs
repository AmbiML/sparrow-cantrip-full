#![no_std]

use core::fmt::Write;
use cstr_core::CStr;
use cantrip_io as io;

// Console logging interface.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
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
        let output: &mut dyn io::Write = &mut self::Tx::new();
        let _ = writeln!(output, "{}", CStr::from_ptr(msg).to_str().unwrap());
    }
}

const DATAPORT_SIZE: usize = 4096;

pub struct Rx {
    dataport: &'static [u8],
}
impl Default for Rx {
    fn default() -> Self { Self::new() }
}

impl Rx {
    pub fn new() -> Rx {
        extern "C" {
            static rx_dataport: *mut cty::c_uchar;
        }
        Rx {
            dataport: unsafe { core::slice::from_raw_parts(rx_dataport, DATAPORT_SIZE) },
        }
    }
}

impl io::Read for Rx {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        extern "C" {
            fn uart_read_read(limit: cty::size_t) -> cty::c_int;
        }
        let n = unsafe { uart_read_read(buf.len()) };
        if n >= 0 {
            let s = n as usize;
            buf[..s].copy_from_slice(&self.dataport[..s]);
            Ok(s)
        } else {
            Err(io::Error)
        }
    }
}

pub struct Tx {
    dataport: &'static mut [u8],
}
impl Default for Tx {
    fn default() -> Self { Self::new() }
}

impl Tx {
    pub fn new() -> Tx {
        extern "C" {
            static tx_dataport: *mut cty::c_uchar;
        }
        Tx {
            dataport: unsafe { core::slice::from_raw_parts_mut(tx_dataport, DATAPORT_SIZE) },
        }
    }
}

impl io::Write for Tx {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        extern "C" {
            fn uart_write_write(available: cty::size_t) -> cty::c_int;
        }
        self.dataport[..buf.len()].copy_from_slice(buf);
        let n = unsafe { uart_write_write(buf.len()) };
        if n >= 0 {
            Ok(n as usize)
        } else {
            Err(io::Error)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        extern "C" {
            fn uart_write_flush() -> cty::c_int;
        }
        if unsafe { uart_write_flush() } == 0 {
            Ok(())
        } else {
            Err(io::Error)
        }
    }
}
