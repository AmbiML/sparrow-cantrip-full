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

use cantrip_os_common::sel4_sys;
use core::time::Duration;
use log::trace;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_NBWait;
use sel4_sys::seL4_Wait;

use static_assertions::const_assert_eq;

pub const TIMERS_PER_CLIENT: usize = 32;

pub type Ticks = u64;
pub type TimerId = u32;
pub type TimerDuration = u32;
pub type TimerMask = u32;

/// A hardware timer capable of generating interrupts.
pub trait HardwareTimer {
    fn setup(&self);
    fn ack_interrupt(&self);
    // The current value of the timer.
    fn now(&self) -> Ticks;
    // Return the deadline `duration` in the future, in Ticks.
    fn deadline(&self, duration: Duration) -> Ticks;
    fn set_alarm(&self, deadline: Ticks);
}

pub trait TimerInterface {
    fn add_oneshot(
        &mut self,
        client_id: usize,
        timer_id: TimerId,
        duration: Duration,
    ) -> Result<(), TimerServiceError>;
    fn add_periodic(
        &mut self,
        client_id: usize,
        timer_id: TimerId,
        duration: Duration,
    ) -> Result<(), TimerServiceError>;
    fn cancel(&mut self, client_id: usize, timer_id: TimerId) -> Result<(), TimerServiceError>;
    fn completed_timers(&mut self, client_id: usize) -> Result<TimerMask, TimerServiceError>;
    fn service_interrupt(&mut self);
}

/// Return codes from TimerService api's.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TimerServiceError {
    TseTimerOk = 0,
    TseNoSuchTimer,
    TseTimerAlreadyExists,
    TseDeserializeFailed,
    TseSerializeFailed,
}
impl From<TimerServiceError> for Result<(), TimerServiceError> {
    fn from(err: TimerServiceError) -> Result<(), TimerServiceError> {
        if err == TimerServiceError::TseTimerOk {
            Ok(())
        } else {
            Err(err)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TimerServiceRequest {
    // Returns a bit vector, where a 1 in bit N indicates timer N has finished.
    // Outstanding completed timers are reset to 0 during this call.
    CompletedTimers, // -> uint32_t

    Oneshot {
        timer_id: TimerId,
        duration_in_ms: TimerDuration,
    },
    Periodic {
        timer_id: TimerId,
        duration_in_ms: TimerDuration,
    },
    Cancel(TimerId),

    Capscan,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletedTimersResponse {
    pub timer_mask: TimerMask,
}

// Size of the data buffer used to pass a serialized TimerServiceRequest
// between Rust <> C. The data structure size is bounded by the camkes ipc
// buffer (120 bytes!) and also by it being allocated on the stack of the rpc
// glue code.
const TIMER_REQUEST_DATA_SIZE: usize = core::mem::size_of::<TimerServiceRequest>();
// Size of the serialized response.
// NB: has to be 'pub const' for bindgen to pickup; also can't use size_of to
// compute it, hence the assert after.
pub const TIMER_RESPONSE_DATA_SIZE: usize = 4;
const_assert_eq!(
    TIMER_RESPONSE_DATA_SIZE,
    core::mem::size_of::<CompletedTimersResponse>()
);
pub type TimerServiceResponseData = [u8; TIMER_RESPONSE_DATA_SIZE];

#[inline]
pub fn cantrip_timer_request<T: DeserializeOwned>(
    request: &TimerServiceRequest,
) -> Result<T, TimerServiceError> {
    // NB: this assumes the SecurityCoordinator component is named "security".
    extern "C" {
        pub fn timer_request(
            c_request_buffer_len: u32,
            c_request_buffer: *const u8,
            c_reply: *mut TimerServiceResponseData,
        ) -> TimerServiceError;
    }
    trace!("cantrip_timer_request {:?}", request);
    let mut request_buffer = [0u8; TIMER_REQUEST_DATA_SIZE];
    let request_slice = postcard::to_slice(request, &mut request_buffer)
        .or(Err(TimerServiceError::TseSerializeFailed))?;
    let mut reply_buffer = [0u8; TIMER_RESPONSE_DATA_SIZE];
    match unsafe {
        timer_request(
            request_slice.len() as u32,
            request_slice.as_ptr(),
            &mut reply_buffer as *mut _,
        )
    } {
        TimerServiceError::TseTimerOk => {
            let reply = postcard::from_bytes(&reply_buffer)
                .or(Err(TimerServiceError::TseDeserializeFailed))?;
            Ok(reply)
        }
        err => Err(err),
    }
}

/// Returns a TimerId bitmask of timers registered with cantrip_timer_oneshot
/// and cantrip_timer_periodic that have expired.
#[inline]
pub fn cantrip_timer_completed_timers() -> Result<TimerMask, TimerServiceError> {
    cantrip_timer_request(&TimerServiceRequest::CompletedTimers)
        .map(|reply: CompletedTimersResponse| reply.timer_mask)
}

/// Registers a one-shot |timer_id| with |duration_in_ms| to start immediately.
/// |timer_id| is interpreted per client and must not be running already.
/// When the timer completes a notification will be delivered to the client.
/// Clients can synchronously wait for this notification using cantrip_timer_wait.
#[inline]
pub fn cantrip_timer_oneshot(
    timer_id: TimerId,
    duration_in_ms: TimerDuration,
) -> Result<(), TimerServiceError> {
    cantrip_timer_request(&TimerServiceRequest::Oneshot {
        timer_id,
        duration_in_ms,
    })
}

/// Registers a periodic |timer_id| with |duration_in_ms| to start immediately.
/// |timer_id| is interpreted per client and must not be running already.
/// When the timer completes a notification will be delivered to the client
/// and another instance of this timer will be automatically started.
/// Clients can synchronously wait for the next notification using
/// cantrip_timer_wait. To stop the timer (and notifications) cantrip_timer_cancel
/// should be called.
#[inline]
pub fn cantrip_timer_periodic(
    timer_id: TimerId,
    duration_in_ms: TimerDuration,
) -> Result<(), TimerServiceError> {
    cantrip_timer_request(&TimerServiceRequest::Periodic {
        timer_id,
        duration_in_ms,
    })
}

/// Stops any pending one-shot or periodic |timer_id|.
#[inline]
pub fn cantrip_timer_cancel(timer_id: TimerId) -> Result<(), TimerServiceError> {
    cantrip_timer_request(&TimerServiceRequest::Cancel(timer_id))
}

/// Returns the cptr for the notification object used to signal timer events.
#[inline]
pub fn cantrip_timer_notification() -> seL4_CPtr {
    extern "C" {
        fn timer_notification() -> seL4_CPtr;
    }
    unsafe { timer_notification() }
}

/// Waits for the next pending timer for the client. If a timer completes
/// the associated timer id is returned.
#[inline]
pub fn cantrip_timer_wait() -> Result<TimerMask, TimerServiceError> {
    unsafe {
        seL4_Wait(cantrip_timer_notification(), core::ptr::null_mut());
    }
    cantrip_timer_completed_timers()
}

/// Returns a bitmask of completed timers. Note this is non-blocking; to
/// wait for one or more timers to complete use cantrip_timer_wait.
#[inline]
pub fn cantrip_timer_poll() -> Result<TimerMask, TimerServiceError> {
    unsafe {
        seL4_NBWait(cantrip_timer_notification(), core::ptr::null_mut());
    }
    cantrip_timer_completed_timers()
}

/// Runs a capscan operation on the TimerService.
#[inline]
pub fn cantrip_timer_capscan() -> Result<(), TimerServiceError> {
    cantrip_timer_request(&TimerServiceRequest::Capscan)
}
