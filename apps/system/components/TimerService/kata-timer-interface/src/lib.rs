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

use core::time::Duration;
use cantrip_os_common::sel4_sys::{seL4_CPtr, seL4_Wait, seL4_Word};

pub type Ticks = u64;
pub type TimerId = u32;

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

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TimerServiceError {
    TimerOk = 0,
    NoSuchTimer,
    TimerAlreadyExists,
}

#[inline]
#[allow(dead_code)]
pub fn timer_service_completed_timers() -> u32 {
    extern "C" {
        fn timer_completed_timers() -> u32;
    }
    unsafe { timer_completed_timers() }
}

#[inline]
#[allow(dead_code)]
pub fn timer_service_oneshot(timer_id: u32, duration_in_ms: u32) -> TimerServiceError {
    extern "C" {
        fn timer_oneshot(timer_id: u32, duration_in_ms: u32) -> TimerServiceError;
    }
    unsafe { timer_oneshot(timer_id, duration_in_ms) }
}

#[inline]
#[allow(dead_code)]
pub fn timer_service_periodic(timer_id: u32, duration_in_ms: u32) -> TimerServiceError {
    extern "C" {
        fn timer_periodic(timer_id: u32, duration_in_ms: u32) -> TimerServiceError;
    }
    unsafe { timer_periodic(timer_id, duration_in_ms) }
}

#[inline]
#[allow(dead_code)]
pub fn timer_service_cancel(timer_id: u32) -> TimerServiceError {
    extern "C" {
        fn timer_cancel(timer_id: u32) -> TimerServiceError;
    }
    unsafe { timer_cancel(timer_id) }
}

#[inline]
#[allow(dead_code)]
pub fn timer_service_notification() -> seL4_CPtr {
    extern "C" {
        fn timer_notification() -> seL4_CPtr;
    }
    unsafe { timer_notification() }
}

#[inline]
#[allow(dead_code)]
pub fn timer_service_wait() -> seL4_Word {
    let mut notification_badge: seL4_Word = 0;

    unsafe {
        seL4_Wait(timer_service_notification(), &mut notification_badge);
    }

    notification_badge
}

#[inline]
#[allow(dead_code)]
pub fn timer_service_capscan() -> Result<(), TimerServiceError> {
    extern "C" {
        fn timer_capscan();
    }
    unsafe { timer_capscan() }
    Ok(())
}
