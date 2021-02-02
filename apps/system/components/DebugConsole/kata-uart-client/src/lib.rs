#![no_std]

use cantrip_io as io;

// C interface to external UART driver.
extern "C" {
    static rx_dataport: *mut cty::c_char;
    static tx_dataport: *mut cty::c_char;
    fn uart_rx(n: cty::size_t);
    fn uart_tx(n: cty::size_t);
}

pub struct Rx {}

impl io::Read for Rx {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        unsafe {
            uart_rx(buf.len());
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
            uart_tx(buf.len());
        }
        Ok(buf.len())
    }
}
