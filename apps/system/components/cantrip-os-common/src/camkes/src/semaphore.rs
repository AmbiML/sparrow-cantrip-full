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

use crate::baresema::seL4_BareSema;
use sel4_sys::seL4_CPtr;

#[macro_export]
macro_rules! static_semaphore {
    ($sem_tag:ident) => {
        static_semaphore!($sem_tag, stringify!($sem_tag));
    };
    ($sem_tag:ident, $sem_name:expr) => {
        crate::paste! {
            pub static [<$sem_tag: upper>]: seL4_Semaphore =
                seL4_Semaphore::new($sem_name, [<$sem_tag:upper _ENDPOINT>], 1);
        }
    };
}

#[derive(Debug)]
pub struct seL4_Semaphore {
    sem: seL4_BareSema,
}
impl seL4_Semaphore {
    pub const fn new(name: &'static str, endpoint: seL4_CPtr, value: isize) -> Self {
        Self {
            sem: seL4_BareSema::new(name, endpoint, value),
        }
    }
    pub fn name(&self) -> &str { self.sem.name() }

    /// Waits (non-blocking) for a semaphore.
    pub fn try_wait(&self) { self.sem.try_wait(); }

    /// Waits (blocking) for a semaphore.
    pub fn wait(&self) { self.sem.wait(); }

    /// Releases a semaphore, signaling waiters.
    pub fn post(&self) { self.sem.post(); }
}
