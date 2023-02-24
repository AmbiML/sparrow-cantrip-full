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
#![allow(stable_features)]
#![feature(map_first_last)]
#![feature(const_btree_new)]

use cantrip_timer_interface::*;
use core::time::Duration;
use spin::Mutex;
use spin::MutexGuard;

mod timer_manager;
pub use timer_manager::TimerManager;

pub struct CantripTimerService<HT> {
    manager: Mutex<Option<TimerManager<HT>>>,
}
impl<HT: HardwareTimer> CantripTimerService<HT> {
    pub const fn empty() -> CantripTimerService<HT> {
        CantripTimerService {
            manager: Mutex::new(None),
        }
    }

    pub fn get(&self) -> Guard<HT> {
        Guard {
            manager: self.manager.lock(),
        }
    }
}
pub struct Guard<'a, HT> {
    manager: MutexGuard<'a, Option<TimerManager<HT>>>,
}
impl<'a, HT: HardwareTimer> Guard<'a, HT> {
    pub fn is_empty(&self) -> bool { self.manager.is_none() }

    pub fn init(&mut self, timer: HT) {
        assert!(self.manager.is_none());
        *self.manager = Some(TimerManager::new(timer));
    }
}
impl<'a, HT: HardwareTimer> TimerInterface for Guard<'a, HT> {
    fn add_oneshot(
        &mut self,
        client_id: usize,
        timer_id: TimerId,
        duration: Duration,
    ) -> Result<(), TimerServiceError> {
        self.manager
            .as_mut()
            .unwrap()
            .add_oneshot(client_id, timer_id, duration)
    }
    fn add_periodic(
        &mut self,
        client_id: usize,
        timer_id: TimerId,
        duration: Duration,
    ) -> Result<(), TimerServiceError> {
        self.manager
            .as_mut()
            .unwrap()
            .add_periodic(client_id, timer_id, duration)
    }
    fn cancel(&mut self, client_id: usize, timer_id: TimerId) -> Result<(), TimerServiceError> {
        self.manager.as_mut().unwrap().cancel(client_id, timer_id)
    }
    fn completed_timers(&mut self, client_id: usize) -> Result<TimerMask, TimerServiceError> {
        self.manager.as_mut().unwrap().completed_timers(client_id)
    }
    fn service_interrupt(&mut self) { self.manager.as_mut().unwrap().service_interrupt() }
}
