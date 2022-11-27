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
#![feature(map_first_last)]
#![feature(const_btree_new)]

use cantrip_timer_interface::*;
use core::time::Duration;
use spin::Mutex;

mod timer_manager;
pub use timer_manager::TimerManager;

pub struct CantripTimerService {
    manager: Mutex<Option<TimerManager>>,
}
impl CantripTimerService {
    pub const fn empty() -> CantripTimerService {
        CantripTimerService {
            manager: Mutex::new(None),
        }
    }
    pub fn init(&self, timer: impl HardwareTimer + Sync + 'static) {
        *self.manager.lock() = Some(TimerManager::new(timer));
    }
}
impl TimerInterface for CantripTimerService {
    fn add_oneshot(
        &mut self,
        client_id: usize,
        timer_id: TimerId,
        duration: Duration,
    ) -> Result<(), TimerServiceError> {
        self.manager
            .lock()
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
            .lock()
            .as_mut()
            .unwrap()
            .add_periodic(client_id, timer_id, duration)
    }
    fn cancel(&mut self, client_id: usize, timer_id: TimerId) -> Result<(), TimerServiceError> {
        self.manager
            .lock()
            .as_mut()
            .unwrap()
            .cancel(client_id, timer_id)
    }
    fn completed_timers(&mut self, client_id: usize) -> Result<TimerMask, TimerServiceError> {
        self.manager
            .lock()
            .as_mut()
            .unwrap()
            .completed_timers(client_id)
    }
    fn service_interrupt(&mut self) { self.manager.lock().as_mut().unwrap().service_interrupt() }
}
