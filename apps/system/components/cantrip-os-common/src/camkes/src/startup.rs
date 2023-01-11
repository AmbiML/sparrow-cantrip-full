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

use crate::Camkes;
use crate::CamkesThreadInterface;

/// Thread startup for a CAmkES component.
///
/// A CAmkES component is a single executable that is started once for each
/// thread. All threads share the same VSpace and CSpace but have per-thread
/// local storage (TLS) and seL4_IPCBuffer. A pre-assigned thread id,
/// supplied by the rootserver, is used to select the thread's "identity".
/// All threads  may run application code. There is one Control thread that
/// manages the startup of the component, all other threads are termed
/// Interface threads because they typically implement the server side of an
/// API interface. The Control thread invokes pre_init & post_init methods
/// before calling run. Interface threads have init & run methods that are
/// sequenced by the Control thread such that: pre_init < {init*} < post_init < {run*}.
/// Interface threads may also be "active" or "passive". An active interface
/// thread has it's own seL4 scheduling context and is always ready to run.
/// A passive interface thread shares or borrows a scheduling context and runs
/// only when bound to a context.
///
/// This startup code is responsible for:
/// 1. Setting up Thread Local Storage (TLS) and the seL4 IPCBuffer used by
///    system calls.
/// 2. Sequencing the per-thread pre_init, init, post_init, and run calls.
///
/// TODO: add support for passive interface threads.
use core::ptr;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_IPCBuffer;

// Target-architecture specific support (please keep sorted)
#[cfg_attr(target_arch = "aarch64", path = "arch/aarch64.rs")]
#[cfg_attr(
    all(target_arch = "arm", target_pointer_width = "32"),
    path = "arch/aarch32.rs"
)]
#[cfg_attr(target_arch = "riscv32", path = "arch/riscv32.rs")]
#[cfg_attr(target_arch = "riscv64", path = "arch/riscv64.rs")]
#[cfg_attr(target_arch = "x86", path = "arch/x86.rs")]
#[cfg_attr(target_arch = "x86_64", path = "arch/x86_64.rs")]
mod arch;
pub use arch::CONFIG_SEL4RUNTIME_STATIC_TLS;

// NB 16 known good across all supported arch's (per upstream)
#[repr(C, align(16))]
pub struct StaticTLS {
    pub data: [u8; CONFIG_SEL4RUNTIME_STATIC_TLS],
}

pub enum CamkesThread {
    Control(
        seL4_CPtr,
        &'static str,
        &'static seL4_IPCBuffer,
        &'static StaticTLS,
        &'static Camkes,
    ),
    Interface(
        seL4_CPtr,
        &'static str,
        &'static seL4_IPCBuffer,
        &'static StaticTLS,
        &'static Camkes,
    ),
    PassiveInterface(
        seL4_CPtr,
        &'static str,
        &'static seL4_IPCBuffer,
        &'static StaticTLS,
        &'static Camkes,
    ),
}
impl CamkesThread {
    #[inline]
    pub fn thread_id(&self) -> seL4_CPtr {
        match self {
            CamkesThread::Control(thread_id, ..)
            | CamkesThread::Interface(thread_id, ..)
            | CamkesThread::PassiveInterface(thread_id, ..) => *thread_id,
        }
    }
    #[inline]
    pub fn tcb(&self) -> seL4_CPtr { self.thread_id() as seL4_CPtr }
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            CamkesThread::Control(_, name, ..)
            | CamkesThread::Interface(_, name, ..)
            | CamkesThread::PassiveInterface(_, name, ..) => *name,
        }
    }
    #[inline]
    pub fn is_control(&self) -> bool {
        match self {
            CamkesThread::Control(..) => true,
            CamkesThread::Interface(..) | CamkesThread::PassiveInterface(..) => false,
        }
    }
    #[inline]
    pub fn is_passive(&self) -> bool {
        match self {
            CamkesThread::PassiveInterface(..) => true,
            CamkesThread::Control(..) | CamkesThread::Interface(..) => false,
        }
    }
    #[inline(never)]
    unsafe fn init_tls(&self) {
        match self {
            CamkesThread::Control(_, _, _, tls, ..)
            | CamkesThread::Interface(_, _, _, tls, ..)
            | CamkesThread::PassiveInterface(_, _, _, tls, ..) => {
                // Sets up TLS for a CAmkES thread. There is a static block
                // of memory just big enough to hold one item: a pointer to
                // the thread's seL4_IPCBuffer used to make system calls.
                // XXX check tbss size from ELF?
                // XXX cheat, can't use addr_of_mut on tls
                arch::set_tls_base(ptr::addr_of!(tls.data[0]));
            }
        }
    }
    #[inline(never)]
    unsafe fn init_ipc_buffer(&self) {
        match self {
            CamkesThread::Control(_, _, ipc_buffer, ..)
            | CamkesThread::Interface(_, _, ipc_buffer, ..)
            | CamkesThread::PassiveInterface(_, _, ipc_buffer, ..) => {
                #[thread_local]
                #[no_mangle] // sel4-sys still uses extern "C"
                static mut __sel4_ipc_buffer: *mut seL4_IPCBuffer = 0 as _;
                __sel4_ipc_buffer = core::mem::transmute(*ipc_buffer);
            }
        }
    }
}

pub trait CamkesThreadStart {
    fn start(thread: &CamkesThread) -> !;
}
impl<T: CamkesThreadInterface> CamkesThreadStart for T {
    fn start(thread: &CamkesThread) -> ! {
        match thread {
            CamkesThread::Control(_, thread_name, .., &ref camkes) => {
                unsafe {
                    // NB: beware of optimizer using tls before setup
                    thread.init_tls();
                    thread.init_ipc_buffer();
                }
                T::pre_init();
                camkes.pre_init_sync();
                T::post_init();
                camkes.post_init_sync();
                T::run();
                log::trace!(target: camkes.name, "{}::run returned", thread_name);
                camkes.interface_init.wait(); // blocking
                unreachable!();
            }
            CamkesThread::Interface(_, thread_name, .., &ref camkes) => {
                camkes.pre_init.wait(); // Wait for Control::pre_init to complete.
                unsafe {
                    // NB: beware of optimizer using tls before setup
                    thread.init_tls();
                    thread.init_ipc_buffer();
                }
                T::init();
                camkes.interface_init.post(); // Signal Control we've completed init.
                camkes.post_init.wait(); // Wait for Control::post_init to complete.
                T::run();
                log::trace!(target: camkes.name, "{}::run returned", thread_name);
                camkes.pre_init.wait(); // blocking
                unreachable!();
            }
            CamkesThread::PassiveInterface(_, _thread_name, .., &ref _camkes) => {
                todo!()
            }
        }
    }
}
/// Synchronization helpers for CamkesThreadStart::start. These are run on
/// the component's control thread (see above).
impl Camkes {
    /// Handles synchronization of interface threads after the control
    /// thread's pre_init method runs.
    pub fn pre_init_sync(&self) {
        // Wake all the non-passive interface threads.
        self.threads
            .iter()
            .filter(|t| !t.is_control() && !t.is_passive())
            .for_each(|_| self.pre_init.post());
        // Wait for the non-passive threads to complete their init work.
        self.threads
            .iter()
            .filter(|t| !t.is_control() && !t.is_passive())
            .for_each(|_| self.interface_init.wait());
        // Wake each passive thread one at a time and allow it to run its init.
        // NB: control threads are never passive so no need to handle specially
        self.threads
            .iter()
            .filter(|t| t.is_passive())
            .for_each(|_t| {
                //seL4_SchedContext_Bind(/*? sc_passive_init ?*/, t.tcb());
                self.pre_init.post();
                self.interface_init.wait();
                //seL4_SchedContext_Unbind(/*? sc_passive_init ?*/);
            });
    }

    /// Handles synchronization of interface threads after the control
    /// thread's post_init method runs.
    pub fn post_init_sync(&self) {
        // Wake all the interface threads, including passive threads.
        // Passive threads will receive the IPC despite not having scheduling contexts
        // at this point. When they are given a scheduling context below they will be
        // unblocked.
        self.threads
            .iter()
            .filter(|t| !t.is_control())
            .for_each(|_| self.post_init.post());

        // Tempororily bind a scheduling context to each passive thread
        // and allow it to start waiting on an endpoint. Threads will
        // indicate that they are ready to have their sc unbound when
        // they send on the init notification. */
        self.threads
            .iter()
            .filter(|t| t.is_passive())
            .for_each(|_t| {
                //seL4_SchedContext_Bind(/*? sc_passive_init ?*/, t.tcb());
                //seL4_Wait(/*? ntfn_passive_init ?*/, ptr::null());
                //seL4_SchedContext_Unbind(/*? sc_passive_init ?*/);
            });
    }
}

/// Macros for generating a static instance of a thread. These are used in the
/// startup code with matching decl & impls in the component's Rust implementation.
/// For example, in camkes.rs this is generated:
///
/// static_interface_thread!(
///    /*name=*/ write,
///    /*tcb=*/ SELF_TCB_WRITE,
///    /*ipc_buffer=*/ core::ptr::addr_of!(_camkes_ipc_buffer_uart_driver_write_0000.data[4096]),
///    &CAMKES
/// );
///
/// while in the component this is used:
///
/// struct ReadInterfaceThread;
/// impl CamkesThreadInterface for ReadInterfaceThread {
///     fn run() -> ! {
///         rpc_basic_recv!(Read, READ_REQUEST_DATA_SIZE, UartDriverError::Success);
///     }
/// }
///
/// Threads for handling IRQ's have a slightly different implementation to hide
/// IRQ processing boilerplate. In camkes.rs this is generated:
///
/// static_irq_thread!(
///    /*name=*/ tx_empty,
///    /*tcb=*/ SELF_TCB_TX_EMPTY,
///    /*ipc_buffer=*/ core::ptr::addr_of!(_camkes_ipc_buffer_uart_driver_tx_empty_0000.data[4096]),
///    &CAMKES
/// );
///
/// while the component has something like this:
///
/// struct TxEmptyInterfaceThread;
/// impl TxEmptyInterfaceThread {
///     fn handler() -> bool {
///         ...
///        true
///    }
/// }
///
/// IRQ handlers use the return value to signal whether processing is complete
/// and the IRQ should be acknowledged. If the caller does not ack the IRQ, the
/// component implementation is required to do it.

#[macro_export]
macro_rules! _static_thread {
    ($variant:ident, $name:ident, $tcb:expr, $ipc_buffer:expr, $camkes:expr) => {
        crate::paste! {
            static [<$name:upper _ $variant:upper _TLS>]: StaticTLS =
                StaticTLS { data: [0u8; CONFIG_SEL4RUNTIME_STATIC_TLS] };
            static [<$name:upper _THREAD>]: CamkesThread =
                CamkesThread::$variant(
                    $tcb,
                    stringify!($name),
                    unsafe { &*($ipc_buffer as *mut sel4_sys::seL4_IPCBuffer) },
                    &[<$name:upper _ $variant:upper _TLS>],
                    $camkes,
                );
        }
    };
    (@IRQ $irq_name:ident, $tcb:expr, $ipc_buffer:expr, $camkes:expr) => {
        crate::paste! {
            static [<$irq_name:upper _INTERFACE_TLS>]: StaticTLS =
                StaticTLS { data: [0u8; CONFIG_SEL4RUNTIME_STATIC_TLS] };
            static [<$irq_name:upper _THREAD>]: CamkesThread =
                CamkesThread::Interface(
                    $tcb,
                    stringify!($irq_name),
                    unsafe { &*($ipc_buffer as *mut sel4_sys::seL4_IPCBuffer) },
                    &[<$irq_name:upper _INTERFACE_TLS>],
                    $camkes,
                );
            // TODO(sleffler): move to a static_irq macro?
            impl CamkesThreadInterface for [<$irq_name:camel InterfaceThread>] {
                fn run() {
                    loop {
                        [<$irq_name:upper _IRQ>].wait();
                        if Self::handler() {
                            [<$irq_name:upper _IRQ>].acknowledge();
                        }
                    }
                }
            }
        }
    };
    (@FAULT $name:ident, $tcb:expr, $ipc_buffer:expr, $camkes:expr) => {
        crate::paste! {
            static FAULT_HANDLER_TLS: StaticTLS =
                StaticTLS { data: [0u8; CONFIG_SEL4RUNTIME_STATIC_TLS] };
            static FAULT_HANDLER_THREAD: CamkesThread =
                CamkesThread::Interface(
                    $tcb,
                    stringify!([<$name _fault_handler>]),
                    unsafe { &*($ipc_buffer as *mut sel4_sys::seL4_IPCBuffer) },
                    &FAULT_HANDLER_TLS,
                    $camkes,
                );
        }
    };
}
#[macro_export]
macro_rules! static_control_thread {
    ($name:ident, $tcb:expr, $ipc_buffer:expr, $camkes:expr) => {
        $crate::_static_thread!(Control, $name, $tcb, $ipc_buffer, $camkes);
    };
}
#[macro_export]
macro_rules! static_irq_thread {
    ($irq_name:ident, $tcb:expr, $ipc_buffer:expr, $camkes:expr) => {
        $crate::_static_thread!(@IRQ $irq_name, $tcb, $ipc_buffer, $camkes);
    };
}
#[macro_export]
macro_rules! static_fault_handler_thread {
    ($name:ident, $tcb:expr, $ipc_buffer:expr, $camkes:expr) => {
        $crate::_static_thread!(@FAULT $name, $tcb, $ipc_buffer, $camkes);
    };
}
#[macro_export]
macro_rules! static_interface_thread {
    ($name:ident, $tcb:expr, $ipc_buffer:expr, $camkes:expr) => {
        $crate::_static_thread!(Interface, $name, $tcb, $ipc_buffer, $camkes);
    };
}
#[macro_export]
macro_rules! static_passive_interface_thread {
    ($name:ident, $tcb:expr, $ipc_buffer:expr, $camkes:expr) => {
        $crate::_static_thread!(PassiveInterface, $name, $tcb, $ipc_buffer, $camkes);
    };
}
