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
#![allow(non_snake_case)]

use cantrip_os_common::camkes;
use cantrip_os_common::logger;
use log::{error, trace};
use mailbox_interface::*;

use camkes::*;
use logger::*;

#[allow(dead_code)]
mod mailbox;
use mailbox::*;

// Generated code...
mod generated {
    include!(concat!(env!("SEL4_OUT_DIR"), "/../mailbox_driver/camkes.rs"));
}
use generated::*;

/// The high bit of the message header is used to distinguish between "inline"
/// messages that fit in the mailbox and "long" messages that contain the
/// physical address of a memory page containing the message.
pub const HEADER_FLAG_LONG_MESSAGE: u32 = 0x80000000;

struct MailboxDriverControlThread;
impl CamkesThreadInterface for MailboxDriverControlThread {
    fn pre_init() {
        // XXX how to handle "maybe" inclusion
        static_logger!(logger);
    }
    // XXX HACK: compensate for rtirq not setup
    fn post_init() { RtirqInterfaceThread::post_init(); }
    fn run() {
        // NB: do not handle rtirq, it blocks waiting for the api thread
        shared_irq_loop!(
            irq,
            wtirq => WtirqInterfaceThread::handler,
            eirq => EirqInterfaceThread::handler
        );
    }
}

// IRQ Support.

// WTIRQ: interrupt for outbox.count > write_threshold.
struct WtirqInterfaceThread;
impl WtirqInterfaceThread {
    fn handler() -> bool {
        trace!("handle {:?}", &WTIRQ_IRQ);
        // We don't have anything to do here yet, so just clear the interrupt.
        set_intr_state(IntrState::new().with_wtirq(true));
        true
    }
}

// RTIRQ: interrupt for inbox.count > read_threshold.
struct RtirqInterfaceThread;
impl RtirqInterfaceThread {
    // XXX not called 'cuz not part of trait impl
    fn post_init() {
        // We always want our receive interrupt to fire as soon as anything appears
        // in the mailbox, so set the threshold to 0.
        set_rirq_threshold(RirqThreshold::new().with_th(0));
        set_intr_state(IntrState::new().with_rtirq(true));
        set_intr_enable(IntrEnable::new().with_rtirq(true));
    }
    fn handler() -> bool {
        trace!("handle {:?}", &RTIRQ_IRQ);
        // Unblock anyone waiting for a message. api_receive() will ack
        // the interrupt once the message contents have been received.
        RX_SEMAPHORE.post();
        false // NB: suppress acknowledge
    }
    pub fn clear() {
        // The interrupt that raised the semaphore has been handled, clear it.
        set_intr_state(IntrState::new().with_rtirq(true));
        RTIRQ_IRQ.acknowledge();
    }
}

// EIRQ: interrupt when an error occurs.
struct EirqInterfaceThread;
impl EirqInterfaceThread {
    fn handler() -> bool {
        let error = get_error();
        error!("{:?}: read {} write {}", &EIRQ_IRQ, error.read(), error.write());
        // We don't have anything to do here yet, so just clear the interrupt.
        set_intr_state(IntrState::new().with_eirq(true));
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
        Self::enqueue(size | HEADER_FLAG_LONG_MESSAGE);
        Self::enqueue(paddr);
        Ok(0)
    }

    // Receives a message from the security core. Blocks the calling thread until an
    // RTIRQ is received indicating a message has arrived.
    fn recv_request(reply_buffer: &mut [u8]) -> Result<usize, MailboxError> {
        RX_SEMAPHORE.wait(); // Wait for message received interrupt
        let header = Self::dequeue();
        let paddr = Self::dequeue();
        // The interrupt that raised the semaphore has been handled, clear it.
        RtirqInterfaceThread::clear();

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

    // Directly manipulate the hardware FIFOs. Synchronous and busy-waits.
    // Not thread-safe (NB: current usage is single-threaded).

    fn enqueue(x: u32) {
        while get_status().full() {}
        set_mboxw(x);
    }
    fn dequeue() -> u32 {
        while get_status().empty() {}
        get_mboxr()
    }
}
