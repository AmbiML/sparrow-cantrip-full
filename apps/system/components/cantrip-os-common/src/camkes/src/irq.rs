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

use core::ptr;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_IRQHandler_Ack;
use sel4_sys::seL4_IRQHandler_Clear;
use sel4_sys::seL4_IRQHandler_SetNotification;
use sel4_sys::seL4_Poll;
use sel4_sys::seL4_Wait;

pub trait seL4IRQInterface {
    fn register(&self);
    fn unregister(&self);
    fn poll(&self) -> bool;
    fn wait(&self);
    fn acknowledge(&self);
}

#[macro_export]
macro_rules! static_irq {
    ($irq_tag:ident) => {
        static_irq!($irq_tag, stringify!($irq_tag));
    };
    ($irq_tag:ident, $irq_name:expr) => {
        crate::paste! {
            #[derive(Debug)]
            struct [<$irq_tag:camel Irq>] {
                pub irq: seL4_IRQ,
            }
            impl seL4IRQInterface for [<$irq_tag:camel Irq>] {
                fn register(&self) { self.irq.register(); }
                fn unregister(&self) { self.irq.unregister(); }
                fn poll(&self) -> bool { self.irq.poll() }
                fn wait(&self) { self.irq.wait(); }
                fn acknowledge(&self) { self.irq.acknowledge(); }
            }
            static [<$irq_tag:upper _IRQ>]: [<$irq_tag:camel Irq>] = [<$irq_tag:camel Irq>] {
                irq: seL4_IRQ::new(
                    $irq_name,
                    [<$irq_tag:upper _NUMBER>],
                    [<$irq_tag:upper _HANDLER>],
                    [<$irq_tag:upper _NOTIFICATION>],
                ),
            };
        }
    };
}

#[derive(Debug)]
pub struct seL4_IRQ {
    name: &'static str,
    number: usize,           // IRQ hardware number
    handler: seL4_CPtr,      // IRQ handler object
    notification: seL4_CPtr, // IRQ notification object
}
impl seL4_IRQ {
    pub const fn new(
        name: &'static str,
        number: usize,
        handler: seL4_CPtr,
        notification: seL4_CPtr,
    ) -> Self {
        Self {
            name,
            number,
            handler,
            notification,
        }
    }
    pub fn name(&self) -> &str { self.name }
    pub fn number(&self) -> usize { self.number }

    /// Registers the irq with kernel. When an irq fires a signal
    /// will be posted to the notification object.
    pub fn register(&self) {
        unsafe { seL4_IRQHandler_SetNotification(self.handler, self.notification) }
            .expect(self.name);
    }

    /// Removes any kernel regisration. No notifications will be posted
    /// by the kernel until a handler is registered.
    pub fn unregister(&self) { unsafe { seL4_IRQHandler_Clear(self.handler) }.expect(self.name); }

    /// Polls (non-blocking) for a pending irq.
    pub fn poll(&self) -> bool {
        unsafe {
            let mut sender = 0;
            seL4_Poll(self.notification, &mut sender);
            sender != 0
        }
    }

    /// Waits (blocking) for an irq.
    pub fn wait(&self) {
        unsafe {
            seL4_Wait(self.notification, ptr::null_mut());
        }
    }

    /// Acknowledges completion of an irq.
    pub fn acknowledge(&self) { unsafe { seL4_IRQHandler_Ack(self.handler) }.expect(self.name); }
}
