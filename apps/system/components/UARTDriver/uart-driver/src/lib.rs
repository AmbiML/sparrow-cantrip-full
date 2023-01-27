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
#![allow(clippy::missing_safety_doc)]

// Include bindings for OpenTitan UART register definition (opentitan/uart.h).
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

mod register;

use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::sel4_sys;
use core::cmp;
use sel4_sys::seL4_PageBits;

// TODO(chrisphan): Use ringbuf crate instead.
use circular_buffer::Buffer;
use register::{bit, Field, Register};

// Frequency of the primary clock clk_i.
//
// TODO: OpenTitan actually specifies 24Mhz, but using that results
// in Renode reporting double the expected BaudRate.
//
// https://docs.opentitan.org/hw/ip/clkmgr/doc/
const CLK_FIXED_FREQ_HZ: u64 = 48_000_000;

// The TX/RX Fifo capacity mentioned in the programming guide.
const UART_FIFO_CAPACITY: u32 = 32;
const BAUD_RATE: u64 = 115200;

// This is the default in CAmkES 2 and the configurable default in CAmkES 3.
const TX_RX_DATAPORT_CAPACITY: usize = 1 << seL4_PageBits;

// Driver-owned circular buffer to receive more than the FIFO size before the
// received data is consumed by rx_update.
static mut RX_BUFFER: Buffer = Buffer::new();

// Driver-owned circular buffer to buffer more transmitted bytes than can fit
// in the transmit FIFO.
static mut TX_BUFFER: Buffer = Buffer::new();

static mut CAMKES: Camkes = Camkes::new("UARTDriver");

extern "C" {
    static rx_dataport: *mut u8;
    static tx_dataport: *mut u8;
    fn rx_mutex_lock() -> u32;
    fn rx_mutex_unlock() -> u32;
    fn tx_mutex_lock() -> u32;
    fn tx_mutex_unlock() -> u32;
    fn rx_empty_semaphore_wait() -> u32;
    fn rx_nonempty_semaphore_wait() -> u32;
    fn rx_nonempty_semaphore_post() -> u32;
    fn rx_watermark_acknowledge() -> u32;
    fn tx_watermark_acknowledge() -> u32;
    fn tx_empty_acknowledge() -> u32;
}

/// Assert while preserving expr in non-debug mode.
#[inline(never)]
fn cantrip_assert(expr: bool) {
    debug_assert!(expr);
}

/// Performs initial programming of the OpenTitan UART at mmio_region.
#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    CAMKES.init_logger(log::LevelFilter::Trace);

    // Computes NCO value corresponding to baud rate.
    // nco = 2^20 * baud / fclk  (assuming NCO width is 16-bit)
    assert_eq!(UART_CTRL_NCO_MASK, 0xffff);
    let ctrl_nco: u64 = (BAUD_RATE << 20) / CLK_FIXED_FREQ_HZ;
    assert!(ctrl_nco < 0xffff);

    // Sets baud rate and enables TX and RX.
    let baud_rate = Field::new(UART_CTRL_NCO_MASK, UART_CTRL_NCO_OFFSET, Some(ctrl_nco as u32));
    Register::new(UART_CTRL_REG_OFFSET)
        .write(*baud_rate | bit(UART_CTRL_TX_BIT) | bit(UART_CTRL_RX_BIT));

    // Resets TX and RX FIFOs.
    let mut fifo_ctrl = Register::new(UART_FIFO_CTRL_REG_OFFSET);
    fifo_ctrl
        .write(fifo_ctrl.get() | bit(UART_FIFO_CTRL_RXRST_BIT) | bit(UART_FIFO_CTRL_TXRST_BIT));

    set_fifo_watermarks();
}

/// Initializes watermarks.
unsafe fn set_fifo_watermarks() {
    let mut fifo_ctrl = Register::new(UART_FIFO_CTRL_REG_OFFSET);
    // Clears old values of both watermarks.
    let mut fifo_ctrl_watermark = fifo_ctrl.get()
        & (!(UART_FIFO_CTRL_RXILVL_MASK << UART_FIFO_CTRL_RXILVL_OFFSET))
        & (!(UART_FIFO_CTRL_TXILVL_MASK << UART_FIFO_CTRL_TXILVL_OFFSET));

    // RX watermark to 1.
    //
    // This enables calls that block on a single byte at a time, like the one
    // the shell does when reading a line of input, to return immediately when
    // that byte is received.
    //
    // Note that this high watermark is only a threshold for when to be informed
    // that bytes have been received. The FIFO can still fill to its full
    // capacity (32) independent of how this is set.
    //
    // Although a higher watermark in combination with rx_timeout might be
    // preferable, Renode simulation does not yet support the rx_timeout
    // interrupt.
    fifo_ctrl_watermark |= *Field::new(
        UART_FIFO_CTRL_RXILVL_MASK,
        UART_FIFO_CTRL_RXILVL_OFFSET,
        Some(UART_FIFO_CTRL_RXILVL_VALUE_RXLVL1),
    );

    // TX watermark to 16 (half full).
    fifo_ctrl_watermark |= *Field::new(
        UART_FIFO_CTRL_TXILVL_MASK,
        UART_FIFO_CTRL_TXILVL_OFFSET,
        Some(UART_FIFO_CTRL_TXILVL_VALUE_TXLVL16),
    );
    fifo_ctrl.write(fifo_ctrl_watermark);

    // Enables interrupts.
    Register::new(UART_INTR_ENABLE_REG_OFFSET).write(
        bit(UART_INTR_COMMON_TX_WATERMARK_BIT)
            | bit(UART_INTR_COMMON_RX_WATERMARK_BIT)
            | bit(UART_INTR_COMMON_TX_EMPTY_BIT),
    );
}

/// Implements Rust Read::read().
///
/// Reads up to a given limit of bytes into the CAmkES rx_dataport, blocking
/// until at least one byte is available.
#[no_mangle]
pub unsafe extern "C" fn read_inf_read(limit: usize) -> isize {
    if limit > TX_RX_DATAPORT_CAPACITY {
        return -1;
    }

    rx_mutex_lock();
    while RX_BUFFER.is_empty() {
        rx_mutex_unlock();
        cantrip_assert(rx_nonempty_semaphore_wait() == 0);
        rx_mutex_lock();
    }

    let mut num_read = 0;
    let dataport = core::slice::from_raw_parts_mut(rx_dataport, limit);
    while num_read < limit {
        if let Some(result) = RX_BUFFER.pop() {
            dataport[num_read] = result;
        } else {
            break;
        }
        num_read += 1;
    }
    rx_mutex_unlock();
    // TODO: Return error code if num_read == 0.
    num_read as isize
}

/// Implements Rust Write::write().
///
/// Writes as many bytes from tx_dataport as the hardware will accept, but not
/// more than the number available (specified by the argument). Returns the
/// number of bytes written or a negative value if there is any error.
#[no_mangle]
pub unsafe extern "C" fn write_inf_write(available: usize) -> isize {
    if available > TX_RX_DATAPORT_CAPACITY {
        return -1;
    }
    let mut num_written = 0;
    let dataport = core::slice::from_raw_parts(tx_dataport, available);
    while num_written < available {
        tx_mutex_lock();
        if !TX_BUFFER.push(dataport[num_written]) {
            tx_mutex_unlock();
            break;
        }
        num_written += 1;
        tx_mutex_unlock();
    }
    fill_tx_fifo();
    // TODO: Return error code if num_written == 0.
    num_written as isize
}

/// Implements Rust Write::flush().
///
/// Drains TX_BUFFER and tx_fifo. Blocks until buffer is empty.
/// Always returns 0.
#[no_mangle]
pub unsafe extern "C" fn write_inf_flush() -> i32 {
    tx_mutex_lock();
    while !TX_BUFFER.is_empty() {
        fill_tx_fifo();
    }
    tx_mutex_unlock();
    0
}

/// Handles a tx_watermark interrupt.
///
/// These happen when the transmit FIFO is half-empty. This refills the FIFO to
/// prevent stalling, stopping early if TX_BUFFER becomes empty, and then signals
/// any tx_update that might be waiting for TX_BUFFER to not be full.
#[no_mangle]
pub unsafe extern "C" fn tx_watermark_handle() {
    fill_tx_fifo();

    // Clears INTR_STATE for tx_watermark. (INTR_STATE is write-1-to-clear.) No
    // similar check to the one in tx_empty_handle is necessary here, since
    // tx_empty will eventually assert and cause anything left in TX_BUFFER
    // to be flushed out.
    Register::new(UART_INTR_STATE_REG_OFFSET).write(bit(UART_INTR_STATE_TX_WATERMARK_BIT));
    cantrip_assert(tx_watermark_acknowledge() == 0);
}

/// Handles an rx_watermark interrupt.
///
/// Reads any bytes currently pending in the receive FIFO into RX_BUFFER,
/// stopping early if RX_BUFFER becomes full and then signals any call
/// rx_update that may be waiting on the condition that RX_BUFFER not be empty.
#[no_mangle]
pub unsafe extern "C" fn rx_watermark_handle() {
    rx_mutex_lock();
    while !rx_empty() {
        let available_data = RX_BUFFER.available_data() as u32;
        if available_data == 0 {
            // The buffer is full.
            //
            // We want to stay in this invocation of the interrupt handler until
            // the RX FIFO is empty, since the rx_watermark interrupt will not
            // fire again until the RX FIFO level crosses from 0 to 1. Therefore
            // we unblock any pending reads and wait for enough reads to consume
            // all of RX_BUFFER.
            cantrip_assert(rx_nonempty_semaphore_post() == 0);
            rx_mutex_unlock();
            cantrip_assert(rx_empty_semaphore_wait() == 0);
            rx_mutex_lock();
            continue;
        }
        let to_read = cmp::min(rx_fifo_level(), available_data);
        for _ in 0..to_read {
            cantrip_assert(RX_BUFFER.push(uart_getchar()));
        }
    }
    cantrip_assert(rx_nonempty_semaphore_post() == 0);
    rx_mutex_unlock();

    Register::new(UART_INTR_STATE_REG_OFFSET).write(bit(UART_INTR_STATE_RX_WATERMARK_BIT));
    cantrip_assert(rx_watermark_acknowledge() == 0);
}

/// Handles a tx_empty interrupt.
///
/// This copies TX_BUFFER into the hardware transmit FIFO, stopping early
/// if TX_BUFFER becomes empty, and then signals any tx_update that might
/// be waiting for TX_BUFFER to not be full.
#[no_mangle]
pub unsafe extern "C" fn tx_empty_handle() {
    fill_tx_fifo();
    tx_mutex_lock();
    if TX_BUFFER.is_empty() {
        // Clears INTR_STATE for tx_empty. (INTR_STATE is write-1-to-clear.) We
        // only do this if TX_BUFFER is empty, since the TX FIFO might have
        // become empty in the time from fill_tx_fifo having sent the last
        // character until here. In that case, we want the interrupt to
        // reassert.
        Register::new(UART_INTR_STATE_REG_OFFSET).write(bit(UART_INTR_STATE_TX_EMPTY_BIT));
    }
    tx_mutex_unlock();
    cantrip_assert(tx_empty_acknowledge() == 0);
}

/// Copies from TX_BUFFER into the transmit FIFO.
///
/// This stops when the transmit FIFO is full or when TX_BUFFER is empty,
/// whichever comes first.
unsafe fn tx_fifo_level() -> u32 {
    let field = Field::new(UART_FIFO_STATUS_TXLVL_MASK, UART_FIFO_STATUS_TXLVL_OFFSET, None);
    Register::new(UART_FIFO_STATUS_REG_OFFSET).read(field)
}

/// Copies from TX_BUFFER into the transmit FIFO.
///
/// This stops when the transmit FIFO is full or when TX_BUFFER is empty,
/// whichever comes first.
unsafe fn fill_tx_fifo() {
    tx_mutex_lock();
    while tx_fifo_level() < UART_FIFO_CAPACITY {
        if let Some(result) = TX_BUFFER.pop() {
            let field =
                Field::new(UART_WDATA_WDATA_MASK, UART_WDATA_WDATA_OFFSET, Some(result as u32));
            Register::new(UART_WDATA_REG_OFFSET).write(*field);
        } else {
            break;
        }
    }
    tx_mutex_unlock();
}

/// Gets whether the receive FIFO empty status bit is set.
///
/// Prefer this to FIFO_STATUS.RXLVL, which the simulation has sometimes
/// reported as zero even when "not STATUS.RXEMPTY."
unsafe fn rx_empty() -> bool {
    Register::new(UART_STATUS_REG_OFFSET).get() & bit(UART_STATUS_RXEMPTY_BIT) != 0
}

/// Gets the number of unread bytes in the RX FIFO from hardware MMIO.
unsafe fn rx_fifo_level() -> u32 {
    let field = Field::new(UART_FIFO_STATUS_RXLVL_MASK, UART_FIFO_STATUS_RXLVL_OFFSET, None);
    Register::new(UART_FIFO_STATUS_REG_OFFSET).read(field)
}

/// Reads one byte from the hardware read data register.
///
/// Callers should first ensure the receive FIFO is not empty rather than rely
/// on any particular magic value to indicate that.
unsafe fn uart_getchar() -> u8 {
    let field = Field::new(UART_RDATA_RDATA_MASK, UART_RDATA_RDATA_OFFSET, None);
    Register::new(UART_RDATA_REG_OFFSET).read(field) as u8
}
