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

mod opentitan_timer;

use cantrip_timer_interface::{HardwareTimer, Ticks};
use core::time::Duration;
use opentitan_timer::*;

// TODO(jesionowski): Grab frequency from top_matcha.h.
const TIMER_BASE_FREQ: u32 = 24_000_000;
const TIMER_FREQ: u32 = 10_000;
const PRESCALE: u16 = ((TIMER_BASE_FREQ / TIMER_FREQ) - 1) as u16;

pub struct OtTimer;

impl HardwareTimer for OtTimer {
    fn setup(&self) {
        opentitan_timer::set_config(Config::new().with_prescale(PRESCALE).with_step(1));
        opentitan_timer::set_compare_high(0);
        opentitan_timer::set_value_low(0xFFFF_0000);
        opentitan_timer::set_intr_state(Intr::new().with_timer0(true)); // w1c
        opentitan_timer::set_intr_enable(Intr::new().with_timer0(false));
        opentitan_timer::set_ctrl(Ctrl::new().with_enable(true));
    }

    fn ack_interrupt(&self) {
        opentitan_timer::set_intr_state(Intr::new().with_timer0(true));
        opentitan_timer::set_intr_enable(Intr::new().with_timer0(false));
    }

    fn now(&self) -> Ticks {
        let low: u32 = opentitan_timer::get_value_low();
        let high: u32 = opentitan_timer::get_value_high();

        ((high as u64) << 32) | low as u64
    }

    fn deadline(&self, duration: Duration) -> Ticks {
        let tick_duration = (TIMER_FREQ as u64 * duration.as_millis() as u64) / 1000;
        self.now() + tick_duration
    }

    fn set_alarm(&self, deadline: Ticks) {
        let high = (deadline >> 32) as u32;
        let low = (deadline & 0xffffffff) as u32;

        // Recommended approach for setting the two compare registers
        // (RISC-V Privileged Architectures 3.1.15)
        opentitan_timer::set_compare_low(0xffffffff);
        opentitan_timer::set_compare_high(high);
        opentitan_timer::set_compare_low(low);

        opentitan_timer::set_intr_enable(Intr::new().with_timer0(true));
    }
}
