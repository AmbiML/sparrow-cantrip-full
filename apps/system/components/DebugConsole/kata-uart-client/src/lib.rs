#![no_std]

use core::fmt::Write;
use cstr_core::CStr;
use cantrip_io as io;

// C interface to external UART driver.
extern "C" {
    static rx_dataport: *mut cty::c_uchar;
    static tx_dataport: *mut cty::c_uchar;
    fn uart_rx_update(n: cty::size_t);
    fn uart_tx_update(n: cty::size_t);
}

// Console logging interface.
#[no_mangle]
pub extern "C" fn logger_log(msg: *const cstr_core::c_char) {
    // TODO(sleffler): is the uart driver ok w/ multiple writers?
    let output: &mut dyn io::Write = &mut self::Tx {};
    unsafe {
        let _ = writeln!(output, "{}", CStr::from_ptr(msg).to_str().unwrap());
    }
}

pub struct Rx {}

impl io::Read for Rx {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        unsafe {
            uart_rx_update(buf.len());
            let port = core::slice::from_raw_parts(rx_dataport, buf.len());
            buf.copy_from_slice(&port);
        }
        Ok(buf.len())
    }
}

pub struct Tx {}

impl io::Write for Tx {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        unsafe {
            let port = core::slice::from_raw_parts_mut(tx_dataport, buf.len());
            port.copy_from_slice(buf);
            uart_tx_update(buf.len());
        }
        Ok(buf.len())
    }
}
