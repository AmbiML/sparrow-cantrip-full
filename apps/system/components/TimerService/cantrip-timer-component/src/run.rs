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
#![allow(clippy::missing_safety_doc)]

use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::sel4_sys::seL4_Word;
use cantrip_timer_interface::CompletedTimersResponse;
use cantrip_timer_interface::TimerId;
use cantrip_timer_interface::TimerInterface;
use cantrip_timer_interface::TimerServiceError;
use cantrip_timer_interface::TimerServiceRequest;
use cantrip_timer_interface::TimerServiceResponseData;
use cantrip_timer_service::CantripTimerService;
use core::slice;
use core::time::Duration;

extern "C" {
    fn timer_get_sender_id() -> seL4_Word;
}

static mut CAMKES: Camkes = Camkes::new("TimerService");

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

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static mut HEAP_MEMORY: [u8; 4 * 1024] = [0; 4 * 1024];
    CAMKES.pre_init(log::LevelFilter::Debug, &mut HEAP_MEMORY);
}

#[no_mangle]
pub unsafe extern "C" fn timer_request(
    c_reques_buffer_len: u32,
    c_request_buffer: *const u8,
    c_reply_buffer: *mut TimerServiceResponseData,
) -> TimerServiceError {
    let _cleanup = Camkes::cleanup_request_cap();
    let request_buffer = slice::from_raw_parts(c_request_buffer, c_reques_buffer_len as usize);
    let request = match postcard::from_bytes::<TimerServiceRequest>(request_buffer) {
        Ok(request) => request,
        Err(_) => return TimerServiceError::TseDeserializeFailed,
    };

    match request {
        TimerServiceRequest::CompletedTimers => completed_timers_request(&mut *c_reply_buffer),

        TimerServiceRequest::Oneshot {
            timer_id,
            duration_in_ms,
        } => oneshot_request(timer_id, duration_in_ms),
        TimerServiceRequest::Periodic {
            timer_id,
            duration_in_ms,
        } => periodic_request(timer_id, duration_in_ms),
        TimerServiceRequest::Cancel(timer_id) => cancel_request(timer_id),

        TimerServiceRequest::Capscan => {
            capscan_request();
            Ok(())
        }
    }
    .map_or_else(|e| e, |()| TimerServiceError::TseTimerOk)
}

fn completed_timers_request(
    reply_buffer: &mut TimerServiceResponseData,
) -> Result<(), TimerServiceError> {
    let client_id = unsafe { timer_get_sender_id() };
    let timer_mask = cantrip_timer().completed_timers(client_id)?;
    let _ = postcard::to_slice(&CompletedTimersResponse { timer_mask }, reply_buffer)
        .or(Err(TimerServiceError::TseSerializeFailed))?;
    Ok(())
}

fn oneshot_request(timer_id: TimerId, duration_ms: u32) -> Result<(), TimerServiceError> {
    let duration = Duration::from_millis(duration_ms as u64);
    let client_id = unsafe { timer_get_sender_id() };
    cantrip_timer().add_oneshot(client_id, timer_id, duration)
}

fn periodic_request(timer_id: TimerId, duration_ms: u32) -> Result<(), TimerServiceError> {
    let duration = Duration::from_millis(duration_ms as u64);
    let client_id = unsafe { timer_get_sender_id() };
    cantrip_timer().add_periodic(client_id, timer_id, duration)
}

fn cancel_request(timer_id: TimerId) -> Result<(), TimerServiceError> {
    let client_id = unsafe { timer_get_sender_id() };
    cantrip_timer().cancel(client_id, timer_id)
}

fn capscan_request() { let _ = Camkes::capscan(); }

#[no_mangle]
pub unsafe extern "C" fn timer_interrupt_handle() {
    extern "C" {
        fn timer_interrupt_acknowledge() -> u32;
    }
    cantrip_timer().service_interrupt();
    assert!(timer_interrupt_acknowledge() == 0);
}
