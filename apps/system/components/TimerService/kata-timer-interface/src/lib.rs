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
#![allow(dead_code)]

use core::time::Duration;
use cantrip_os_common::sel4_sys;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_NBWait;
use sel4_sys::seL4_Wait;

pub const TIMERS_PER_CLIENT: usize = 32;

pub type Ticks = u64;
pub type TimerId = u32;
pub type TimerMask = u32;
pub type TimerDuration = u32;

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

/// Return codes from TimerService api's.
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TimerServiceError {
    TimerOk = 0,
    NoSuchTimer,
    TimerAlreadyExists,
}
impl From<TimerServiceError> for Result<(), TimerServiceError> {
    fn from(err: TimerServiceError) -> Result<(), TimerServiceError> {
        if err == TimerServiceError::TimerOk {
            Ok(())
        } else {
            Err(err)
        }
    }
}

/// Returns a TimerId bitmask of timers registered with cantrip_timer_oneshot
/// and cantrip_timer_periodic that have expired.
#[inline]
pub fn cantrip_timer_completed_timers() -> Result<TimerMask, TimerServiceError> {
    extern "C" {
        fn timer_completed_timers() -> u32;
    }
    Ok(unsafe { timer_completed_timers() } as TimerMask)
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
    extern "C" {
        fn timer_oneshot(timer_id: u32, duration_in_ms: u32) -> TimerServiceError;
    }
    unsafe { timer_oneshot(timer_id as u32, duration_in_ms as u32) }.into()
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
    extern "C" {
        fn timer_periodic(timer_id: u32, duration_in_ms: u32) -> TimerServiceError;
    }
    unsafe { timer_periodic(timer_id as u32, duration_in_ms as u32) }.into()
}

/// Stops any pending one-shot or periodic |timer_id|.
#[inline]
pub fn cantrip_timer_cancel(timer_id: TimerId) -> Result<(), TimerServiceError> {
    extern "C" {
        fn timer_cancel(timer_id: u32) -> TimerServiceError;
    }
    unsafe { timer_cancel(timer_id as u32) }.into()
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
    extern "C" {
        fn timer_capscan();
    }
    unsafe { timer_capscan() }
    Ok(())
}
