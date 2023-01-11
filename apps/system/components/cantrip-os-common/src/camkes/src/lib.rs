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

//! Cantrip OS CAmkES component helpers

#![no_std]
#![allow(non_camel_case_types)]
#![feature(c_variadic)]
#![feature(thread_local)]

#[cfg(feature = "libc_compat")]
pub mod compat;

use allocator;
use core::ops::Deref;
use log::trace;
use slot_allocator::CANTRIP_CSPACE_SLOTS;
use spin::Mutex;

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_GetCap;
use sel4_sys::seL4_GetCapReceivePath;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_SetCap;
use sel4_sys::seL4_SetCapReceivePath;
use sel4_sys::seL4_Word;
use sel4_sys::seL4_WordBits;

pub mod baresema;
pub mod irq;
pub use paste::*; // re-export for macros
pub mod rpc_basic;
pub mod rpc_shared;
pub mod semaphore;
pub mod startup;
pub use startup::CamkesThread;

pub type seL4_CPath = (seL4_CPtr, seL4_CPtr, seL4_Word);

extern "Rust" {
    static SELF_CNODE: seL4_CPtr;
    static SELF_CNODE_FIRST_SLOT: seL4_CPtr;
    static SELF_CNODE_LAST_SLOT: seL4_CPtr;
}

// RAII wrapper for handling request cap cleanup.
pub struct RequestCapCleanup {}
impl Drop for RequestCapCleanup {
    fn drop(&mut self) { set_cap(0); }
}

// RAII wrapper for releasing reply cap after use.
pub struct ReplyCapRelease {
    cpath: seL4_CPath,
}
impl Drop for ReplyCapRelease {
    fn drop(&mut self) {
        // XXX check cap is unchanged
        set_cap(0);
        unsafe {
            CANTRIP_CSPACE_SLOTS.free(self.cpath.1, 1);
            Camkes::delete_path(&self.cpath).expect("delete");
        }
    }
}

// RAII wrapper for handling cpath cleanup.
pub struct OwnedCPath {
    name: &'static str, // Component name
    cpath: seL4_CPath,
}
impl Deref for OwnedCPath {
    type Target = seL4_CPath;

    fn deref(&self) -> &Self::Target { &self.cpath }
}
impl Drop for OwnedCPath {
    fn drop(&mut self) {
        // Clears any capability the cpath points to.
        // Assert since future receives are likely to fail
        Camkes::delete_path(&self.cpath).expect(self.name);
    }
}

// Attaches a capability to a CAmkES RPC request/reply msg.
// seL4 will copy the capabiltiy.
// NB: private because the api is unsafe, set_request_cap
//   and set_reply_cap_release should always be used
#[inline]
fn set_cap(cptr: seL4_CPtr) { unsafe { seL4_SetCap(0, cptr) } }
#[inline]
fn get_cap() -> seL4_CPtr { unsafe { seL4_GetCap(0) } }

/// Callbacks from startup. This is how services are bound to the
/// thread model.
pub trait CamkesThreadInterface {
    fn pre_init() {} // NB: control thread only
    fn post_init() {} // NB: control thread only
    fn init() {}
    fn run() {} // XXX generate message as thread is unused?
}

pub struct Camkes {
    name: &'static str, // Component name
    pre_init: &'static baresema::seL4_BareSema,
    post_init: &'static baresema::seL4_BareSema,
    interface_init: &'static baresema::seL4_BareSema,
    threads: &'static [&'static CamkesThread],
    recv_path: Mutex<seL4_CPath>, // IPCBuffer receive path
}
impl Camkes {
    pub const fn new(
        name: &'static str,
        pre_init: &'static baresema::seL4_BareSema,
        post_init: &'static baresema::seL4_BareSema,
        interface_init: &'static baresema::seL4_BareSema,
        threads: &'static [&CamkesThread],
    ) -> Self {
        Self {
            name,
            pre_init,
            post_init,
            interface_init,
            threads,
            recv_path: Mutex::new((seL4_CPtr::MAX, seL4_CPtr::MAX, seL4_Word::MAX)),
        }
    }
    pub fn thread(&self, thread_id: usize) -> Option<&CamkesThread> {
        self.threads.iter().find_map(|&t| {
            if t.thread_id() == thread_id {
                Some(t)
            } else {
                None
            }
        })
    }

    pub fn init_allocator(&self, heap: &'static mut [u8]) {
        unsafe {
            allocator::ALLOCATOR.init(heap.as_mut_ptr(), heap.len());
        }
    }

    pub fn init_slot_allocator(&self, first_slot: seL4_CPtr, last_slot: seL4_CPtr) {
        unsafe {
            CANTRIP_CSPACE_SLOTS.init(self.name, first_slot, last_slot - first_slot);
        }
    }

    pub fn pre_init(&self, heap: &'static mut [u8]) {
        self.init_allocator(heap);
        self.init_slot_allocator(unsafe { SELF_CNODE_FIRST_SLOT }, unsafe { SELF_CNODE_LAST_SLOT });
    }

    #[inline]
    pub fn top_level_path(slot: seL4_CPtr) -> seL4_CPath {
        (unsafe { SELF_CNODE }, slot, seL4_WordBits)
    }

    // Initializes the IPCBuffer receive path with |path|.
    pub fn init_recv_path(&self, path: &seL4_CPath) {
        *self.recv_path.lock() = *path;
        unsafe {
            seL4_SetCapReceivePath(path.0, path.1, path.2);
        }
        trace!(target: self.name, "cap receive path {:?}", path);
    }

    // Returns the path specified with init_recv_path.
    pub fn get_recv_path(&self) -> seL4_CPath { *self.recv_path.lock() }

    // Returns the component name.
    #[inline]
    pub fn get_name(&self) -> &'static str { self.name }

    // Returns the current receive path from the IPCBuffer.
    pub fn get_current_recv_path(&self) -> seL4_CPath { unsafe { seL4_GetCapReceivePath() } }

    // Returns the current receive path from the IPCBuffer; clears any
    // capability the cpath points to when dropped.
    #[must_use]
    pub fn get_owned_current_recv_path(&self) -> OwnedCPath {
        // NB: make sure noone clobbers the setup done in init_recv_path
        self.assert_recv_path();
        OwnedCPath {
            name: self.name,
            cpath: self.get_current_recv_path(),
        }
    }

    // Check the current receive path in the IPCBuffer against what was
    // setup with init_recv_path.
    pub fn check_recv_path(&self) -> bool { self.get_current_recv_path() == self.get_recv_path() }

    // Like check_recv_path but asserts if there is an inconsistency.
    pub fn assert_recv_path(&self) {
        assert!(
            self.check_recv_path(),
            "Current receive path {:?} does not match init'd path {:?}",
            self.get_current_recv_path(),
            self.recv_path
        );
    }

    // Deletes the capability at |path|,
    pub fn delete_path(path: &seL4_CPath) -> seL4_Result {
        unsafe { seL4_CNode_Delete(path.0, path.1, path.2 as u8) }
    }

    // Attaches a capability to a CAmkES RPC request MessageInfo and
    // returns a helper to reset/cleanup on block exit. seL4 will copy
    // the capabilty if the MessageInfo indicates there are capabilities
    // attached (beware this is currently buried in the CAmkES template).
    #[must_use]
    pub fn set_request_cap(cptr: seL4_CPtr) -> RequestCapCleanup {
        set_cap(cptr);
        Self::cleanup_request_cap()
    }

    // Arranges for the CAmkES RPC request capability be clear'd on
    // block exit. This is to guard against accidentally attaching a
    // capability to a reply.
    // TODO(sleffler): remove after the C templates are replaced
    #[must_use]
    pub fn cleanup_request_cap() -> RequestCapCleanup { RequestCapCleanup {} }

    // Immediately clears any capability attached to a CAmkES RPC request
    // msg. NB: cleanup_request_cap may be more useful.
    pub fn clear_request_cap() { set_cap(0); }

    // Returns the capability attached to an seL4 IPC.
    pub fn get_request_cap() -> seL4_CPtr { get_cap() }

    // Attaches a capability to a CAmkES RPC reply msg and arranges for
    // resources to be released after the reply completes.
    #[must_use]
    pub fn set_reply_cap_release(cptr: seL4_CPtr) -> ReplyCapRelease {
        set_cap(cptr);
        ReplyCapRelease {
            cpath: Self::top_level_path(cptr),
        }
    }

    // Clears any capability attached to a CAmkES RPC reply msg.
    // XXX dangerous
    pub fn clear_reply_cap() { set_cap(0); }

    // Returns the capability attached to an seL4 IPC.
    pub fn get_reply_cap() -> seL4_CPtr { get_cap() }

    // Wrappers for sel4_sys::debug_assert macros.
    pub fn debug_assert_slot_empty(tag: &str, path: &seL4_CPath) {
        sel4_sys::debug_assert_slot_empty!(
            path.1,
            "{}: expected slot {:?} empty but has cap type {:?}",
            tag,
            path,
            sel4_sys::cap_identify(path.1)
        );
    }
    pub fn debug_assert_slot_cnode(tag: &str, path: &seL4_CPath) {
        sel4_sys::debug_assert_slot_cnode!(
            path.1,
            "{}: expected cnode in slot {:?} but found cap type {:?}",
            tag,
            path,
            sel4_sys::cap_identify(path.1)
        );
    }
    pub fn debug_assert_slot_frame(tag: &str, path: &seL4_CPath) {
        sel4_sys::debug_assert_slot_frame!(
            path.1,
            "{}: expected frame in slot {:?} but found cap type {:?}",
            tag,
            path,
            sel4_sys::cap_identify(path.1)
        );
    }

    // debug_assert wrappers for the current recv_path.
    pub fn debug_assert_recv_path_empty(&self, tag: &str) {
        Self::debug_assert_slot_empty(tag, &self.get_current_recv_path());
    }
    pub fn debug_assert_recv_path_cnode(&self, tag: &str) {
        Self::debug_assert_slot_cnode(tag, &self.get_current_recv_path());
    }
    pub fn debug_assert_recv_path_frame(&self, tag: &str) {
        Self::debug_assert_slot_frame(tag, &self.get_current_recv_path());
    }

    // Dumps the contents of the toplevel CNode to the serial console.
    pub fn capscan() -> seL4_Result {
        // TODO(sleffler): requires CONFIG_PRINTING in the kernel
        #[cfg(feature = "CONFIG_PRINTING")]
        unsafe {
            sel4_sys::seL4_DebugDumpCNode(SELF_CNODE);
        }
        // XXX until seL4_Error is correctly returned
        Ok(())
    }
}
