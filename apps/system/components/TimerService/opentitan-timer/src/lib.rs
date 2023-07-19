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

//! This crate provides access to an OpenTitan timer that satisfies the
//! HardwareTimer interface.
#![no_std]

#[allow(dead_code)]
mod opentitan_timer;
use opentitan_timer::*;

use cantrip_timer_interface::{HardwareTimer, Ticks};
use core::time::Duration;

// Primary clock frequency.
use reg_constants::platform::TOP_MATCHA_SMC_TIMER0_BASE_FREQ_HZ as TIMER_BASE_FREQ;

const TIMER_FREQ: u64 = 10_000;
const PRESCALE: u16 = ((TIMER_BASE_FREQ / TIMER_FREQ) - 1) as u16;

pub struct OtTimer;

impl HardwareTimer for OtTimer {
    fn setup(&self) {
        set_config(Config::new().with_prescale(PRESCALE).with_step(1));
        set_compare_high(0);
        set_value_low(0xFFFF_0000);
        set_intr_status(IntrStatus::new().with_timer0(true)); // w1c
        set_intr_enable(IntrEnable::new().with_timer0(false));
        set_ctrl(Ctrl::new().with_active(true));
    }

    fn ack_interrupt(&self) {
        set_intr_status(IntrStatus::new().with_timer0(true));
        set_intr_enable(IntrEnable::new().with_timer0(false));
    }

    fn now(&self) -> Ticks {
        let low: u32 = get_value_low();
        let high: u32 = get_value_high();

        ((high as u64) << 32) | low as u64
    }

    fn deadline(&self, duration: Duration) -> Ticks {
        let tick_duration = (TIMER_FREQ * duration.as_millis() as u64) / 1000;
        self.now() + tick_duration
    }

    fn set_alarm(&self, deadline: Ticks) {
        let high = (deadline >> 32) as u32;
        let low = (deadline & 0xffffffff) as u32;

        // Recommended approach for setting the two compare registers
        // (RISC-V Privileged Architectures 3.1.15)
        set_compare_low(0xffffffff);
        set_compare_high(high);
        set_compare_low(low);

        set_intr_enable(IntrEnable::new().with_timer0(true));
    }
}
