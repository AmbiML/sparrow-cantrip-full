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

/// Interrupt support. CAmkES components identify a harwdare Interrupt
/// they may generate with:
///
///   component foo {
///     hardware;
///     emits Interrupt src;
///   }
///
/// A component that processes the interrupt is identified with:
///
///   component bar {
///     consumes Interrupt dest;
///   }
///
/// This is then connected in the assembly with:
///
///    connection cantripIRQ foo_bar(from foo.src, to bar.dest);
///
/// The cantripIRQ template creates a static seL4_IRQ instance for each
/// hardware interrupt and either a dedicated thread for each consumer, or
/// a thread that processes multiple (aka shared) IRQ's in one handler. The
/// IRQ event plumbing is typically done by the rootserver before
/// components are even started.
///
/// IRQ events are badged with a value that is unique per-connection so
/// that shared IRQ setups can identify multiple IRQ's per notification.
/// This badging also is done for the 1:1 (dedicated) setup though the
/// badge is not used.

#[derive(Debug)]
pub struct seL4_IRQ {
    name: &'static str,
    number: usize,           // IRQ hardware number
    mask: usize,             // Notification bitmask
    handler: seL4_CPtr,      // IRQ handler object
    notification: seL4_CPtr, // IRQ notification object
}
impl seL4_IRQ {
    pub const fn new(
        name: &'static str,
        number: usize,
        mask: usize,
        handler: seL4_CPtr,
        notification: seL4_CPtr,
    ) -> Self {
        Self {
            name,
            number,
            mask,
            handler,
            notification,
        }
    }
    pub fn name(&self) -> &str { self.name }
    pub fn number(&self) -> usize { self.number }
    pub fn mask(&self) -> usize { self.mask }

    /// Registers the IRQ with the kernel. When an IRQ fires a signal will
    /// be posted to the notification object. Note most IRQ's are registered
    /// by the rootserver so this method is rarely needed.
    pub fn register(&self) {
        unsafe { seL4_IRQHandler_SetNotification(self.handler, self.notification) }
            .expect(self.name);
    }

    /// Removes any kernel regisration. No notifications will be posted
    /// by the kernel until a handler is registered.
    pub fn unregister(&self) { unsafe { seL4_IRQHandler_Clear(self.handler) }.expect(self.name); }

    /// Polls (non-blocking) for a pending IRQ.
    pub fn poll(&self) -> usize {
        let mut mask = 0;
        unsafe {
            seL4_Poll(self.notification, &mut mask);
        }
        mask
    }
    /// Check a bitmask returned by seL4_Wait/seL4_Poll for this IRQ.
    pub fn is_present(&self, mask: usize) -> bool { (mask & self.mask) != 0 }

    /// Waits (blocking) for an IRQ.
    pub fn wait(&self) {
        unsafe {
            seL4_Wait(self.notification, ptr::null_mut());
        }
    }

    /// Acknowledges completion of an IRQ.
    pub fn acknowledge(&self) { unsafe { seL4_IRQHandler_Ack(self.handler) }.expect(self.name); }
}

#[macro_export]
macro_rules! static_irq {
    ($irq_tag:ident, $irq_mask:expr) => {
        // TODO(sleffler): not currently used, remove?
        /// IRQ bound to a dedicated thread.
        crate::paste! {
            static_irq!($irq_tag, $irq_mask, [<$irq_tag:upper _NOTIFICATION>]);
        }
    };
    ($irq_tag:ident, $irq_mask:expr, $irq_notification:expr) => {
        crate::paste! {
            static [<$irq_tag:upper _IRQ>]: seL4_IRQ = seL4_IRQ::new(
                stringify!($irq_tag),
                [<$irq_tag:upper _NUMBER>],
                $irq_mask,
                [<$irq_tag:upper _HANDLER>],
                $irq_notification,
            );
        }
    };
}

/// Main loop for a dedicated IRQ thread (typically invoked by
/// static_irq_thread!).
//
// TODO(sleffler): allowing the handler to disable ack feels brittle; it is
//   there for the MailboxDriver (probably can eliminate, or maybe just
//   open-code that case and make handler return void)
pub fn irq_loop(irq: &seL4_IRQ, handler: fn() -> bool) -> ! {
    loop {
        irq.wait();
        if handler() {
            irq.acknowledge();
        }
    }
}

/// Main loop to handle multiple IRQ's sharing a single notification
/// object. The IRQ's must be badged with unique bitmasks as done by the
/// CAmkES cantripIRQ template; e.g. a specification of the form:
///
/// connection cantripIRQ uart_irq(
///     from uart.tx_watermark,
///     from uart.rx_watermark,
///     from uart.tx_empty,
///     to uart_driver.irq);
///
/// generates:
///     static_irq!(tx_watermark, 1, IRQ_NOTIFICATION);
///     static_irq!(rx_watermark, 2, IRQ_NOTIFICATION);
///     static_irq!(tx_empty, 4, IRQ_NOTIFICATION);
/// which sets up a shared notification object that can be handled with
///     shared_irq_loop!(irq,
///         tx_watermark => TxWatermarkInterfaceThread::handler,
///         rx_watermark => RxWatermarkInterfaceThread::handler,
///         tx_empty => TxEmptyInterfaceThread::handler
///     );
///
#[macro_export]
macro_rules! shared_irq_loop {
    ($irq_tag:ident, $( $irq:ident => $handler:expr ),*) => {
        crate::paste! {
            shared_irq_loop!(@end
                [<$irq_tag:upper _NOTIFICATION>],
                $( [<$irq:upper _IRQ>] => $handler ),*
            )
        }
    };
    (@end $irq_notification:ident, $( $irq:ident => $handler:expr ),*) => {
        loop {
            let mut irq_mask: sel4_sys::seL4_CPtr = 0;
            unsafe {
                sel4_sys::seL4_Wait($irq_notification, &mut irq_mask);
            }
            assert!(irq_mask != 0);
            $(if $irq.is_present(irq_mask) {
                $handler();
                $irq.acknowledge();
                irq_mask &= ! $irq.mask();
            })*
            assert!(irq_mask == 0);
        }
    };
}
