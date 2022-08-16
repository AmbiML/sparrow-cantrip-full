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

#![no_std]
// We want to keep all mailbox constants here even if they're currently unused.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use cantrip_os_common::logger::CantripLogger;
use log::{error, trace};

//------------------------------------------------------------------------------
// TODO(aappleby): Can we replace this with the register_struct! thing?

const REG_INTR_STATE: u32 = 0x000; // R/W1C
const REG_INTR_ENABLE: u32 = 0x004; // R/W
const REG_INTR_TEST: u32 = 0x008; // R/W
const REG_MBOXW: u32 = 0x00C; // W
const REG_MBOXR: u32 = 0x010; // R
const REG_STATUS: u32 = 0x014; // R
const REG_ERROR: u32 = 0x018; // R
const REG_WIRQT: u32 = 0x01C; // R/W
const REG_RIRQT: u32 = 0x020; // R/W
const REG_CTRL: u32 = 0x024; // R/W

const INTR_STATE_BIT_WTIRQ: u32 = 0b001;
const INTR_STATE_BIT_RTIRQ: u32 = 0b010;
const INTR_STATE_BIT_EIRQ: u32 = 0b100;
const INTR_STATE_MASK: u32 = 0b111;

const INTR_ENABLE_BIT_WTIRQ: u32 = 0b001;
const INTR_ENABLE_BIT_RTIRQ: u32 = 0b010;
const INTR_ENABLE_BIT_EIRQ: u32 = 0b100;
const INTR_ENABLE_MASK: u32 = 0b111;

const INTR_TEST_BIT_WTIRQ: u32 = 0b001;
const INTR_TEST_BIT_RTIRQ: u32 = 0b010;
const INTR_TEST_BIT_EIRQ: u32 = 0b100;
const INTR_TEST_MASK: u32 = 0b111;

const STATUS_BIT_EMPTY: u32 = 0b0001;
const STATUS_BIT_FULL: u32 = 0b0010;
const STATUS_BIT_WFIFOL: u32 = 0b0100;
const STATUS_BIT_RFIFOL: u32 = 0b1000;
const STATUS_MASK: u32 = 0b1111;

const ERROR_BIT_READ: u32 = 0b01;
const ERROR_BIT_WRITE: u32 = 0b10;
const ERROR_MASK: u32 = 0b11;

const FIFO_SIZE: u32 = 8;
const FIFO_MASK: u32 = FIFO_SIZE - 1;
const WIRQT_MASK: u32 = FIFO_MASK;
const RIRQT_MASK: u32 = FIFO_MASK;

const CTRL_BIT_FLUSH_WFIFO: u32 = 0b01;
const CTRL_BIT_FLUSH_RFIFO: u32 = 0b10;
const CTRL_MASK: u32 = 0b11;

// The high bit of the message header is used to distinguish between "inline"
// messages that fit in the mailbox and "long" messages that contain the
// physical address of a memory page containing the message.
const HEADER_FLAG_LONG_MESSAGE: u32 = 0x80000000;

//------------------------------------------------------------------------------

extern "C" {
    // Mailbox registers
    static mailbox_mmio: *mut u32;

    // Global mailbox lock
    fn api_mutex_lock() -> u32;
    fn api_mutex_unlock() -> u32;

    // Mailbox arrival semaphore
    fn rx_semaphore_wait() -> u32;
    fn rx_semaphore_post() -> u32;

    // Mailbox interrupts
    fn wtirq_acknowledge() -> u32;
    fn rtirq_acknowledge() -> u32;
    fn eirq_acknowledge() -> u32;
}

//------------------------------------------------------------------------------
// Directly manipulate the mailbox registers.

unsafe fn get_intr_state() -> u32 { mailbox_mmio.offset(0).read_volatile() }
unsafe fn get_INTR_ENABLE() -> u32 { mailbox_mmio.offset(1).read_volatile() }
unsafe fn get_INTR_TEST() -> u32 { mailbox_mmio.offset(2).read_volatile() }
unsafe fn get_MBOXW() -> u32 { mailbox_mmio.offset(3).read_volatile() }
unsafe fn get_MBOXR() -> u32 { mailbox_mmio.offset(4).read_volatile() }
unsafe fn get_STATUS() -> u32 { mailbox_mmio.offset(5).read_volatile() }
unsafe fn get_ERROR() -> u32 { mailbox_mmio.offset(6).read_volatile() }
unsafe fn get_WIRQT() -> u32 { mailbox_mmio.offset(7).read_volatile() }
unsafe fn get_RIRQT() -> u32 { mailbox_mmio.offset(8).read_volatile() }
unsafe fn get_CTRL() -> u32 { mailbox_mmio.offset(9).read_volatile() }

unsafe fn set_INTR_STATE(x: u32) { mailbox_mmio.offset(0).write_volatile(x); }
unsafe fn set_INTR_ENABLE(x: u32) { mailbox_mmio.offset(1).write_volatile(x); }
unsafe fn set_INTR_TEST(x: u32) { mailbox_mmio.offset(2).write_volatile(x); }
unsafe fn set_MBOXW(x: u32) { mailbox_mmio.offset(3).write_volatile(x); }
unsafe fn set_MBOXR(x: u32) { mailbox_mmio.offset(4).write_volatile(x); }
unsafe fn set_STATUS(x: u32) { mailbox_mmio.offset(5).write_volatile(x); }
unsafe fn set_ERROR(x: u32) { mailbox_mmio.offset(6).write_volatile(x); }
unsafe fn set_WIRQT(x: u32) { mailbox_mmio.offset(7).write_volatile(x); }
unsafe fn set_RIRQT(x: u32) { mailbox_mmio.offset(8).write_volatile(x); }
unsafe fn set_CTRL(x: u32) { mailbox_mmio.offset(9).write_volatile(x); }

//------------------------------------------------------------------------------
// Directly manipulate the hardware FIFOs. Synchronous and busy-waits. Not
// thread-safe, should only be used while holding the api_mutex lock.

fn enqueue_u32(x: u32) {
    unsafe {
        while (get_STATUS() & STATUS_BIT_FULL) == STATUS_BIT_FULL {}
        set_MBOXW(x);
    }
}

fn dequeue_u32() -> u32 {
    unsafe {
        while (get_STATUS() & STATUS_BIT_EMPTY) == STATUS_BIT_EMPTY {}
        get_MBOXR()
    }
}

fn drain_read_fifo() {
    unsafe {
        while (get_STATUS() & STATUS_BIT_EMPTY) == 0 {
            let _ = get_MBOXR();
        }
    }
}

//------------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    // We always want our receive interrupt to fire as soon as anything appears
    // in the mailbox, so set the threshold to 0.
    set_RIRQT(0);
    set_INTR_STATE(INTR_STATE_BIT_RTIRQ);
    set_INTR_ENABLE(INTR_ENABLE_BIT_RTIRQ);
}

//------------------------------------------------------------------------------

// When outbox.count > write_threshold, this interrupt fires.
#[no_mangle]
pub unsafe extern "C" fn wtirq_handle() {
    trace!("wtirq_handle()");

    // We don't have anything to do here yet, so just clear the interrupt.
    set_INTR_STATE(INTR_STATE_BIT_WTIRQ);
    wtirq_acknowledge();
}

// When inbox.count > read_threshold, this interrupt fires.
#[no_mangle]
pub unsafe extern "C" fn rtirq_handle() {
    trace!("rtirq_handle()");

    // Unblock anyone waiting for a message. api_receive() below will clear
    // the interrupt once the message has been deliverd to the client.
    rx_semaphore_post();
}

// When an error occurs, this interrupt fires. We don't handle errors yet.
#[no_mangle]
pub unsafe extern "C" fn eirq_handle() {
    error!("eirq_handle() - error flag is 0x{:X}", get_ERROR());

    // We don't have anything to do here yet, so just clear the interrupt.
    set_INTR_STATE(INTR_STATE_BIT_EIRQ);
    eirq_acknowledge();
}

//------------------------------------------------------------------------------

// Send a message to the security core. The message must be at a _physical_
// address, as the security core knows nothing about seL4's virtual memory.
#[no_mangle]
pub unsafe extern "C" fn api_send(request_paddr: u32, request_size: u32) {
    api_mutex_lock();

    let request_header = request_size | HEADER_FLAG_LONG_MESSAGE;
    enqueue_u32(request_header);
    enqueue_u32(request_paddr);

    api_mutex_unlock();
}

// Receive a message from the security core. Blocks the calling thread until a
// message arrives.
#[no_mangle]
pub unsafe extern "C" fn api_receive(response_paddr: *mut u32, response_size: *mut u32) {
    api_mutex_lock();

    // When a message arrives, the interrupt handler will raise the semaphore.
    rx_semaphore_wait();

    // Message arrived, dequeue it.
    let message_header = dequeue_u32();
    let message_paddr = dequeue_u32();
    response_paddr.write(message_paddr);
    response_size.write(message_header & !HEADER_FLAG_LONG_MESSAGE);

    // The interrupt that raised the semaphore has been handled, clear it.
    set_INTR_STATE(INTR_STATE_BIT_RTIRQ);
    rtirq_acknowledge();

    api_mutex_unlock();
}

//------------------------------------------------------------------------------
