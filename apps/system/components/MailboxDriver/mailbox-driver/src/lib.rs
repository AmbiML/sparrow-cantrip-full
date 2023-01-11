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
//error[E0658]: dereferencing raw mutable pointers in statics is unstable
#![feature(const_mut_refs)]
// XXX for camkes.rs
#![allow(dead_code)]
#![allow(unused_unsafe)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use cantrip_os_common::camkes;
use cantrip_os_common::logger;
use cantrip_os_common::sel4_sys;
use log::{error, trace};
use mailbox_interface::*;

use logger::*;

include!("registers.rs");

// Generated code...
include!(concat!(env!("SEL4_OUT_DIR"), "/../mailbox_driver/camkes.rs"));
fn get_mbox() -> *const u32 { unsafe { MAILBOX_MMIO.data.as_ptr() as _ } }
fn get_mbox_mut() -> *mut u32 { unsafe { MAILBOX_MMIO.data.as_mut_ptr() as _ } }

struct MailboxDriverControlThread;
impl CamkesThreadInterface for MailboxDriverControlThread {
    fn pre_init() {
        // XXX how to handle "maybe" inclusion
        static_logger!(logger);
    }
    // XXX HACK: compensate for rtirq not setup
    fn post_init() { RtirqInterfaceThread::post_init(); }
}

// IRQ Support.

// WTIRQ: interrupt for outbox.count > write_threshold.
struct WtirqInterfaceThread;
impl WtirqInterfaceThread {
    fn handler() -> bool {
        trace!("handle {:?}", unsafe { &WTIRQ_IRQ });
        // We don't have anything to do here yet, so just clear the interrupt.
        //        unsafe { set_INTR_STATE(*mbox_mutex.lock(), INTR_STATE_BIT_WTIRQ) };
        let mailbox_mmio = get_mbox_mut();
        unsafe { set_INTR_STATE(mailbox_mmio, INTR_STATE_BIT_WTIRQ) };
        true
    }
}

// RTIRQ: interrupt for inbox.count > read_threshold.
struct RtirqInterfaceThread;
impl RtirqInterfaceThread {
    // XXX not called 'cuz not part of trait impl
    fn post_init() {
        unsafe {
            // We always want our receive interrupt to fire as soon as anything appears
            // in the mailbox, so set the threshold to 0.
            //            let mbox = mbox_mutex.lock();
            let mbox = get_mbox_mut();
            set_RIRQT(mbox, 0);
            set_INTR_STATE(mbox, INTR_STATE_BIT_RTIRQ);
            set_INTR_ENABLE(mbox, INTR_ENABLE_BIT_RTIRQ);
        }
    }
    fn handler() -> bool {
        trace!("handle {:?}", &RTIRQ_IRQ);
        // Unblock anyone waiting for a message. api_receive() below will
        // ack the interrupt once the message has been deliverd to the client.
        RX_SEMAPHORE.post();
        false // NB: suppress acknowledge
    }
    pub unsafe fn clear(mbox: *mut u32) {
        // The interrupt that raised the semaphore has been handled, clear it.
        set_INTR_STATE(mbox, INTR_STATE_BIT_RTIRQ);
        RTIRQ_IRQ.acknowledge();
    }
}

// EIRQ: interrupt when an error occurs.
struct EirqInterfaceThread;
impl EirqInterfaceThread {
    fn handler() -> bool {
        unsafe {
            error!("{:?}: error {:#X}", &EIRQ_IRQ, get_ERROR(get_mbox()));
        }
        //get_ERROR(*mbox_mutex.lock()),

        // We don't have anything to do here yet, so just clear the interrupt.
        //        unsafe { set_INTR_STATE(*mbox_mutex.lock(), INTR_STATE_BIT_EIRQ) };
        unsafe { set_INTR_STATE(get_mbox_mut(), INTR_STATE_BIT_EIRQ) };
        true
    }
}

// API interface support.

struct ApiInterfaceThread;
impl CamkesThreadInterface for ApiInterfaceThread {
    fn run() {
        rpc_basic_recv!(api, MAILBOX_REQUEST_DATA_SIZE, MailboxError::Success);
    }
}
impl ApiInterfaceThread {
    fn dispatch(
        _client_badge: usize,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> Result<usize, MailboxError> {
        let request = match postcard::from_bytes::<MailboxRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(MailboxError::DeserializeFailed),
        };
        match request {
            MailboxRequest::Send(paddr, size) => Self::send_request(paddr, size),
            MailboxRequest::Recv => Self::recv_request(reply_buffer),
        }
    }

    // Sends a message to the security core. The message must be at a _physical_
    // address, as the security core knows nothing about seL4's virtual memory.
    fn send_request(paddr: u32, size: u32) -> Result<usize, MailboxError> {
        //    let mbox = mbox_mutex.lock();
        let mbox = get_mbox_mut();
        Self::enqueue(mbox, size | HEADER_FLAG_LONG_MESSAGE);
        Self::enqueue(mbox, paddr);
        Ok(0)
    }

    // Receives a message from the security core. Blocks the calling thread until a
    // message arrives.
    fn recv_request(reply_buffer: &mut [u8]) -> Result<usize, MailboxError> {
        let (header, paddr) = unsafe {
            RX_SEMAPHORE.wait(); // Wait for message received interrupt
                                 //    let mbox = mbox_mutex.lock();
            let mbox = get_mbox_mut();
            let header = Self::dequeue(mbox);
            let paddr = Self::dequeue(mbox);
            // The interrupt that raised the semaphore has been handled, clear it.
            RtirqInterfaceThread::clear(mbox);
            (header, paddr)
        };

        let reply_slice = postcard::to_slice(
            &RecvResponse {
                paddr,
                size: header & !HEADER_FLAG_LONG_MESSAGE,
            },
            reply_buffer,
        )
        .or(Err(MailboxError::SerializeFailed))?;
        Ok(reply_slice.len())
    }

    // Directly manipulate the hardware FIFOs. Synchronous and busy-waits. Not
    // thread-safe, should only be used while holding the mbox mutex.

    fn enqueue(mbox: *mut u32, x: u32) {
        unsafe {
            while (get_STATUS(mbox) & STATUS_BIT_FULL) == STATUS_BIT_FULL {}
            set_MBOXW(mbox, x);
        }
    }
    fn dequeue(mbox: *const u32) -> u32 {
        unsafe {
            while (get_STATUS(mbox) & STATUS_BIT_EMPTY) == STATUS_BIT_EMPTY {}
            get_MBOXR(mbox)
        }
    }
    #[allow(dead_code)]
    fn drain_read_fifo(mbox: *const u32) {
        unsafe {
            while (get_STATUS(mbox) & STATUS_BIT_EMPTY) == 0 {
                let _ = get_MBOXR(mbox);
            }
        }
    }
}
