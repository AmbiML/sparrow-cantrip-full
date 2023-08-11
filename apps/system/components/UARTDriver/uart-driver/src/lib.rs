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

//! UART driver.

#![no_std]
//error[E0658]: dereferencing raw mutable pointers in statics is unstable
#![feature(const_mut_refs)]

#[allow(dead_code)]
mod uart;
use uart::*;

use cantrip_os_common::camkes;
use cantrip_os_common::logger;
use cantrip_os_common::sel4_sys;
use spin::Mutex;
use uart_interface::*;

use camkes::*;
use logger::*;

use sel4_sys::seL4_PageBits;

// TODO(chrisphan): Use ringbuf crate instead.
use circular_buffer::Buffer;

// Frequency of the primary clock clk_i.
// https://docs.opentitan.org/hw/ip/clkmgr/doc/
use reg_constants::platform::TOP_MATCHA_SMC_UART_CLOCK_FREQ_PERIPHERAL_HZ as CLK_FIXED_FREQ_HZ;

// The TX/RX Fifo capacity mentioned in the programming guide.
const UART_FIFO_CAPACITY: u32 = 32;
const BAUD_RATE: u64 = 115200;

// This is the default in CAmkES 2 and the configurable default in CAmkES 3.
const TX_RX_DATAPORT_CAPACITY: usize = 1 << seL4_PageBits;

// Driver-owned circular buffer to receive more than the FIFO size before the
// received data is consumed by rx_update.
static RX_BUFFER: Mutex<Buffer> = Mutex::new(Buffer::new());

// Driver-owned circular buffer to buffer more transmitted bytes than can fit
// in the transmit FIFO.
static TX_BUFFER: Mutex<Buffer> = Mutex::new(Buffer::new());

// Generated code...
mod generated {
    include!(concat!(env!("SEL4_OUT_DIR"), "/../uart_driver/camkes.rs"));
}
use generated::*;

struct UartDriverControlThread;
impl CamkesThreadInterface for UartDriverControlThread {
    fn pre_init() {
        // XXX how to handle "maybe" inclusion
        static_logger!(logger);

        // Computes NCO value corresponding to baud rate.
        // nco = 2^20 * baud / fclk  (assuming NCO width is 16-bit)
        let ctrl_nco: u64 = (BAUD_RATE << 20) / CLK_FIXED_FREQ_HZ;
        debug_assert!(ctrl_nco <= reg_constants::uart::UART_CTRL_NCO_MASK as u64);

        // Set baud rate and enable TX and RX.
        set_ctrl(
            Ctrl::new()
                .with_nco(ctrl_nco as u16)
                .with_tx(true)
                .with_rx(true),
        );

        // Reset TX and RX FIFOs.
        set_fifo_ctrl(get_fifo_ctrl().with_rxrst(true).with_txrst(true));

        set_fifo_watermarks();
    }
    // NB: use control thread to handle IRQ's. We could put the handlers
    // here but leave them in interface threads in case we want to split
    // them back into dedicated handler threads.
    fn run() {
        shared_irq_loop!(
            irq,
            tx_watermark => TxWatermarkInterfaceThread::handler,
            tx_empty => TxEmptyInterfaceThread::handler
        );
    }
}

// Handles a tx_watermark interrupt.
//
// These happen when the transmit FIFO is half-empty. This refills the FIFO to
// prevent stalling, stopping early if TX_BUFFER becomes empty, and then signals
// any tx_update that might be waiting for TX_BUFFER to not be full.
struct TxWatermarkInterfaceThread;
impl TxWatermarkInterfaceThread {
    fn handler() -> bool {
        fill_tx_fifo();

        // Clears INTR_STATE for tx_watermark. (INTR_STATE is write-1-to-clear.) No
        // similar check to the one in tx_empty_handle is necessary here, since
        // tx_empty will eventually assert and cause anything left in TX_BUFFER
        // to be flushed out.
        set_intr_state(IntrState::new().with_tx_watermark(true));
        true
    }
}

// Handles an rx_watermark interrupt.
//
// Reads any bytes currently pending in the receive FIFO into RX_BUFFER,
// stopping early if RX_BUFFER becomes full and then signals any call
// rx_update that may be waiting on the condition that RX_BUFFER not be empty.
struct RxWatermarkInterfaceThread;
impl RxWatermarkInterfaceThread {
    fn handler() -> bool {
        let mut buf = RX_BUFFER.lock();
        while rx_fifo_level() > 0 {
            if buf.available_data() == 0 {
                // The buffer is full.
                //
                // We want to stay in this invocation of the interrupt handler until
                // the RX FIFO is empty, since the rx_watermark interrupt will not
                // fire again until the RX FIFO level crosses from 0 to 1. Therefore
                // we unblock any pending reads and wait for RX_BUFFER to drain.
                RX_NONEMPTY.post();
                drop(buf);
                RX_EMPTY.wait();
                buf = RX_BUFFER.lock();
                continue;
            }
            while rx_fifo_level() > 0 && buf.available_data() > 0 {
                let _ = buf.push(uart_getchar());
            }
        }
        // NB: RX_BUFFER must be unlocked before posting RX_NONEMPTY (see read_request)
        drop(buf);
        RX_NONEMPTY.post();

        set_intr_state(IntrState::new().with_rx_watermark(true));
        true
    }
}

// Handles a tx_empty interrupt.
//
// This copies TX_BUFFER into the hardware transmit FIFO, stopping early
// if TX_BUFFER becomes empty, and then signals any tx_update that might
// be waiting for TX_BUFFER to not be full.
struct TxEmptyInterfaceThread;
impl TxEmptyInterfaceThread {
    fn handler() -> bool {
        fill_tx_fifo();
        let buf = TX_BUFFER.lock();
        if buf.is_empty() {
            // Clears INTR_STATE for tx_empty. (INTR_STATE is write-1-to-clear.) We
            // only do this if TX_BUFFER is empty, since the TX FIFO might have
            // become empty in the time from fill_tx_fifo having sent the last
            // character until here. In that case, we want the interrupt to
            // reassert.
            set_intr_state(IntrState::new().with_tx_empty(true));
        }
        drop(buf); // XXX drop on block exit?
        true
    }
}

/// Initializes watermarks.
fn set_fifo_watermarks() {
    // Set FIFO watermarks:
    //
    // RX watermark to 1.
    //   This enables calls that block on a single byte at a time, like the one
    //   the shell does when reading a line of input, to return immediately when
    //   that byte is received.
    //
    //   Note that this high watermark is only a threshold for when to be informed
    //   that bytes have been received. The FIFO can still fill to its full
    //   capacity (32) independent of how this is set.
    //
    //   Although a higher watermark in combination with rx_timeout might be
    //   preferable, Renode simulation does not yet support the rx_timeout
    //   interrupt.
    //
    // TX watermark to 16 (half full).
    set_fifo_ctrl(
        get_fifo_ctrl()
            .with_rxilvl(RxILvl::Level1)
            .with_txilvl(TxILvl::Level16),
    );

    // Enables interrupts.
    set_intr_enable(
        IntrEnable::new()
            .with_tx_watermark(true)
            .with_rx_watermark(true)
            .with_tx_empty(true),
    );
}

struct ReadInterfaceThread;
impl CamkesThreadInterface for ReadInterfaceThread {
    fn run() {
        rpc_basic_recv!(Read, READ_REQUEST_DATA_SIZE, UartDriverError::Success);
    }
}
impl ReadInterfaceThread {
    fn dispatch(
        _client_badge: usize,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> Result<usize, UartDriverError> {
        let request = match postcard::from_bytes::<ReadRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(UartDriverError::DeserializeFailed),
        };
        match request {
            ReadRequest::Read(limit) => Self::read_request(limit, reply_buffer),
        }
    }

    /// Implements Rust Read::read().
    ///
    /// Reads up to a given limit of bytes into the CAmkES RX_DATAPORT, blocking
    /// until at least one byte is available.
    fn read_request(limit: usize, reply_buffer: &mut [u8]) -> Result<usize, UartDriverError> {
        if limit > TX_RX_DATAPORT_CAPACITY {
            // XXX why not just truncate
            return Err(UartDriverError::BadLimit);
        }

        let mut buf = RX_BUFFER.lock();
        while buf.is_empty() {
            drop(buf);
            RX_NONEMPTY.wait();
            buf = RX_BUFFER.lock();
        }

        let mut num_read = 0;
        let dataport = unsafe { &mut RX_DATAPORT.data[..limit] };
        while num_read < limit {
            if let Some(result) = buf.pop() {
                dataport[num_read] = result;
            } else {
                break;
            }
            num_read += 1;
        }
        let is_empty = buf.is_empty();
        drop(buf);
        if is_empty {
            RX_EMPTY.post();
        }

        // TODO: Return error code if num_read == 0.
        let reply_slice = postcard::to_slice(&ReadResponse { num_read }, reply_buffer)
            .or(Err(UartDriverError::SerializeFailed))?;
        Ok(reply_slice.len())
    }
}

struct WriteInterfaceThread;
impl CamkesThreadInterface for WriteInterfaceThread {
    fn run() {
        rpc_basic_recv!(write, WRITE_REQUEST_DATA_SIZE, UartDriverError::Success);
    }
}
impl WriteInterfaceThread {
    fn dispatch(
        _client_badge: usize,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> Result<usize, UartDriverError> {
        let request = match postcard::from_bytes::<WriteRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(UartDriverError::DeserializeFailed),
        };
        match request {
            WriteRequest::Write(available) => Self::write_request(available, reply_buffer),
            WriteRequest::Flush => Self::flush_request(),
        }
    }

    /// Implements Rust Write::write().
    ///
    /// Writes as many bytes from TX_DATAPORT as the hardware will accept, but not
    /// more than the number available (specified by the argument). Returns the
    /// number of bytes written or a negative value if there is any error.
    fn write_request(available: usize, reply_buffer: &mut [u8]) -> Result<usize, UartDriverError> {
        if available > TX_RX_DATAPORT_CAPACITY {
            // XXX why not just truncate
            return Err(UartDriverError::BadLimit);
        }
        let mut num_written = 0;
        let dataport = unsafe { &TX_DATAPORT.data[..available] };
        while num_written < available {
            let mut buf = TX_BUFFER.lock();
            if !buf.push(dataport[num_written]) {
                break;
            }
            num_written += 1;
        }
        fill_tx_fifo();
        // TODO: Return error code if num_written == 0.
        let reply_slice = postcard::to_slice(&WriteResponse { num_written }, reply_buffer)
            .or(Err(UartDriverError::SerializeFailed))?;
        Ok(reply_slice.len())
    }

    /// Implements Rust Write::flush().
    ///
    /// Drains TX_BUFFER and tx_fifo. Blocks until buffer is empty.
    fn flush_request() -> Result<usize, UartDriverError> {
        let buf = TX_BUFFER.lock();
        while !buf.is_empty() {
            fill_tx_fifo();
        }
        Ok(0)
    }
}

/// Copies from TX_BUFFER into the transmit FIFO.
///
/// This stops when the transmit FIFO is full or when TX_BUFFER is empty,
/// whichever comes first.
fn tx_fifo_level() -> u32 { get_fifo_status().txlvl().into() }

/// Copies from TX_BUFFER into the transmit FIFO.
///
/// This stops when the transmit FIFO is full or when TX_BUFFER is empty,
/// whichever comes first.
fn fill_tx_fifo() {
    let mut buf = TX_BUFFER.lock();
    while tx_fifo_level() < UART_FIFO_CAPACITY {
        if let Some(result) = buf.pop() {
            set_wdata(result);
        } else {
            break;
        }
    }
    // drop(buf) (happens on block exit)
}

/// Gets the number of unread bytes in the RX FIFO from hardware MMIO.
fn rx_fifo_level() -> u32 { get_fifo_status().rxlvl().into() }

/// Reads one byte from the hardware read data register.
///
/// Callers should first ensure the receive FIFO is not empty rather than rely
/// on any particular magic value to indicate that.
fn uart_getchar() -> u8 { get_rdata() }
