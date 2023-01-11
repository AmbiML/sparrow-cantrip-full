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

//! The Timer Service provides multiplexed access to a hardware timer.

#![no_std]
// XXX for camkes.rs
#![feature(const_mut_refs)]
#![allow(dead_code)]
#![allow(unused_unsafe)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]

use cantrip_os_common::camkes;
use cantrip_os_common::logger;
use cantrip_os_common::sel4_sys;
use cantrip_timer_interface::CompletedTimersResponse;
use cantrip_timer_interface::TimerId;
use cantrip_timer_interface::TimerInterface;
use cantrip_timer_interface::TimerServiceError;
use cantrip_timer_interface::TimerServiceRequest;
use cantrip_timer_interface::TIMER_REQUEST_DATA_SIZE;
use cantrip_timer_service::CantripTimerService;
use core::time::Duration;

use camkes::irq::seL4_IRQ;
use camkes::*;
use logger::*;

// Generated code...
include!(concat!(env!("SEL4_OUT_DIR"), "/../timer_service/camkes.rs"));

fn cantrip_timer() -> impl TimerInterface {
    static CANTRIP_TIMER: CantripTimerService<opentitan_timer::OtTimer> =
        CantripTimerService::empty();
    let mut manager = CANTRIP_TIMER.get();
    if manager.is_empty() {
        #[cfg(feature = "CONFIG_PLAT_SPARROW")]
        manager.init(opentitan_timer::OtTimer);

        #[cfg(not(feature = "CONFIG_PLAT_SPARROW"))]
        panic!("TimerService enabled without hardware timer support!");
    }
    manager
}

struct TimerServiceControlThread;
impl CamkesThreadInterface for TimerServiceControlThread {
    fn pre_init() {
        // XXX how to handle "maybe" inclusion
        static_logger!(logger);

        unsafe {
            static mut HEAP_MEMORY: [u8; 4 * 1024] = [0; 4 * 1024];
            CAMKES.pre_init(&mut HEAP_MEMORY);
        }
    }
}

struct TimerInterruptInterfaceThread;
impl TimerInterruptInterfaceThread {
    fn handler() -> bool {
        cantrip_timer().service_interrupt();
        true
    }
}

struct TimerInterfaceThread;
impl CamkesThreadInterface for TimerInterfaceThread {
    fn run() {
        rpc_basic_recv!(timer, TIMER_REQUEST_DATA_SIZE, TimerServiceError::Success);
    }
}
impl TimerInterfaceThread {
    fn dispatch(
        client_id: usize, //XXX
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> Result<usize, TimerServiceError> {
        let _cleanup = Camkes::cleanup_request_cap();
        let request = match postcard::from_bytes::<TimerServiceRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(TimerServiceError::DeserializeFailed),
        };

        match request {
            TimerServiceRequest::CompletedTimers => {
                Self::completed_timers_request(client_id, reply_buffer)
            }
            TimerServiceRequest::Oneshot {
                timer_id,
                duration_in_ms,
            } => Self::oneshot_request(client_id, timer_id, duration_in_ms),
            TimerServiceRequest::Periodic {
                timer_id,
                duration_in_ms,
            } => Self::periodic_request(client_id, timer_id, duration_in_ms),
            TimerServiceRequest::Cancel(timer_id) => Self::cancel_request(client_id, timer_id),
            TimerServiceRequest::Capscan => Self::capscan_request(),
        }
    }

    fn completed_timers_request(
        client_id: usize,
        reply_buffer: &mut [u8],
    ) -> Result<usize, TimerServiceError> {
        let timer_mask = cantrip_timer().completed_timers(client_id)?;
        let reply_slice = postcard::to_slice(&CompletedTimersResponse { timer_mask }, reply_buffer)
            .or(Err(TimerServiceError::SerializeFailed))?;
        Ok(reply_slice.len())
    }

    fn oneshot_request(
        client_id: usize,
        timer_id: TimerId,
        duration_ms: u32,
    ) -> Result<usize, TimerServiceError> {
        let duration = Duration::from_millis(duration_ms as u64);
        cantrip_timer()
            .add_oneshot(client_id, timer_id, duration)
            .map(|_| 0)
    }

    fn periodic_request(
        client_id: usize,
        timer_id: TimerId,
        duration_ms: u32,
    ) -> Result<usize, TimerServiceError> {
        let duration = Duration::from_millis(duration_ms as u64);
        cantrip_timer()
            .add_periodic(client_id, timer_id, duration)
            .map(|_| 0)
    }

    fn cancel_request(client_id: usize, timer_id: TimerId) -> Result<usize, TimerServiceError> {
        cantrip_timer().cancel(client_id, timer_id).map(|_| 0)
    }

    fn capscan_request() -> Result<usize, TimerServiceError> {
        let _ = Camkes::capscan();
        Ok(0)
    }
}
