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

// Bare-bones semaphore support for startup synchronization.

//XXX static_assertions::assert_cfg!(feature = "CONFIG_KERNEL_MCS");

use core::ptr;
use core::sync::atomic::{AtomicIsize, Ordering};

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Signal;
use sel4_sys::seL4_WaitWithMRs;

#[macro_export]
macro_rules! static_bare_sema {
    ($sem_tag:ident) => {
        static_bare_sema!($sem_tag, stringify!($sem_tag));
    };
    ($sem_tag:ident, $sem_name:expr) => {
        crate::paste! {
            static [<$sem_tag:upper>]: seL4_BareSema =
                seL4_BareSema::new($sem_name, [<$sem_tag:upper _ENDPOINT>], 0);
        }
    };
}

#[derive(Debug)]
pub struct seL4_BareSema {
    name: &'static str,
    endpoint: seL4_CPtr, // Semaphore endpoint object
    count: AtomicIsize,
}
impl seL4_BareSema {
    pub const fn new(name: &'static str, endpoint: seL4_CPtr, value: isize) -> Self {
        Self {
            name,
            endpoint,
            count: AtomicIsize::new(value),
        }
    }
    pub fn name(&self) -> &str { self.name }

    /// Waits (non-blocking) for a semaphore.
    pub fn try_wait(&self) { todo!() }

    /// Waits (blocking) for a semaphore.
    pub fn wait(&self) {
        if self.count.fetch_sub(1, Ordering::Acquire) <= 0 {
            unsafe {
                // XXX can this be seL4_Wait?
                seL4_WaitWithMRs(
                    /*src=*/ self.endpoint,
                    /*sender=*/ ptr::null_mut(),
                    /*mr0=*/ ptr::null_mut(),
                    /*mr1=*/ ptr::null_mut(),
                    /*mr2=*/ ptr::null_mut(),
                    /*mr3=*/ ptr::null_mut(),
                );
            }
            // NB: barrier required after acquiring the lock.
            let _ = self.count.load(Ordering::Acquire);
        }
    }

    /// Releases a semaphore, signaling waiters.
    pub fn post(&self) {
        if (1 + self.count.fetch_add(1, Ordering::Release)) <= 0 {
            unsafe {
                seL4_Signal(self.endpoint);
            }
        }
    }
}
