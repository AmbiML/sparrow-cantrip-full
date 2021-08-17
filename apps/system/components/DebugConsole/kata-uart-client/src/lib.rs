#![no_std]

use core::fmt::Write;
use cstr_core::CStr;
use cantrip_io as io;

// C interface to external UART driver.
extern "C" {
    static rx_dataport: *mut cty::c_uchar;
    fn uart_rx_update(n: cty::size_t);
    fn rx_mutex_lock();
    fn rx_mutex_unlock();

    static tx_dataport: *mut cty::c_uchar;
    fn uart_tx_update(n: cty::size_t);
    fn tx_mutex_lock();
    fn tx_mutex_unlock();
}

// Console logging interface.
#[no_mangle]
pub extern "C" fn logger_log(level: u8, msg: *const cstr_core::c_char) {
    use log::Level;
    // TODO(sleffler): seems like this should be try_from?
    let l = match level {
        x if x == Level::Error as u8 => Level::Error,
        x if x == Level::Warn as u8 => Level::Warn,
        x if x == Level::Info as u8 => Level::Info,
        x if x == Level::Debug as u8 => Level::Debug,
        x if x == Level::Trace as u8 => Level::Trace,
        _ => { return },  // TODO(sleffler): accept or not?
    };
    if l <= log::max_level() {
        // TODO(sleffler): is the uart driver ok w/ multiple writers?
        let output: &mut dyn io::Write = &mut self::Tx {};
        unsafe {
            let _ = writeln!(output, "{}", CStr::from_ptr(msg).to_str().unwrap());
        }
    }
}

pub struct Rx {}

impl io::Read for Rx {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        unsafe {
            rx_mutex_lock();
            uart_rx_update(buf.len());
            let port = core::slice::from_raw_parts(rx_dataport, buf.len());
            buf.copy_from_slice(&port);
            rx_mutex_unlock();
        }
        Ok(buf.len())
    }
}

pub struct Tx {}

impl io::Write for Tx {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        unsafe {
            tx_mutex_lock();
            let port = core::slice::from_raw_parts_mut(tx_dataport, buf.len());
            port.copy_from_slice(buf);
            uart_tx_update(buf.len());
            tx_mutex_unlock();
        }
        Ok(buf.len())
    }
}
