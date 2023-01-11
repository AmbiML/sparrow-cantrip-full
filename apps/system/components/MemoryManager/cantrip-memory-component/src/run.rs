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

//! Cantrip OS MemoryManager component support.

// Code here binds the camkes component to the rust code.

// TODO(sleffler): don't need 2x threads

#![no_std]
// XXX for camkes.rs
#![feature(const_mut_refs)]
#![allow(dead_code)]
#![allow(unused_unsafe)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]

use cantrip_memory_interface::MemoryLifetime;
use cantrip_memory_interface::MemoryManagerError;
use cantrip_memory_interface::MemoryManagerInterface;
use cantrip_memory_interface::MemoryManagerRequest;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_memory_interface::StatsResponse;
use cantrip_memory_interface::MEMORY_REQUEST_DATA_SIZE;
use cantrip_memory_manager::CantripMemoryManager;
use cantrip_os_common::camkes;
use cantrip_os_common::logger;
use cantrip_os_common::sel4_sys;
use core::ops::Range;
use core::ptr;
use log::info;

use camkes::*;
use logger::*;

use sel4_sys::seL4_BootInfo;
use sel4_sys::seL4_CPtr;

// Generated code...
include!(concat!(env!("SEL4_OUT_DIR"), "/../memory_manager/camkes.rs"));

fn cantrip_memory() -> impl MemoryManagerInterface {
    static CANTRIP_MEMORY: CantripMemoryManager = CantripMemoryManager::empty();
    let mut manager = CANTRIP_MEMORY.get();
    if manager.is_empty() {
        // The MemoryManager component is labeled to receive BootInfo; use
        // it to complete initialization of the MemoryManager interface.
        let bootinfo = get_bootinfo();
        let untyped = unsafe { ptr::addr_of!(bootinfo.untyped).read_volatile() };
        manager.init(
            /*slots=*/
            Range::<seL4_CPtr> {
                start: untyped.start,
                end: untyped.end,
            },
            /*untypeds=*/ unsafe { bootinfo.untyped_descs() },
        );
    }
    manager
}

struct MemoryManagerControlThread;
impl CamkesThreadInterface for MemoryManagerControlThread {
    fn pre_init() {
        // XXX how to handle "maybe" inclusion
        static_logger!(logger);

        // XXX MaybeUninit?
        static mut HEAP_MEMORY: [u8; 8 * 1024] = [0; 8 * 1024];
        unsafe {
            CAMKES.init_allocator(&mut HEAP_MEMORY);
        }

        let bootinfo = get_bootinfo();
        if let Ok(stats) = cantrip_memory().stats() {
            info!(
                "Global memory: {} allocated {} free, reserved: {} kernel {} user",
                stats.allocated_bytes,
                stats.free_bytes,
                bootinfo.kernelReservedBytes,
                stats.overhead_bytes,
            );
        }

        unsafe {
            CAMKES.init_slot_allocator(bootinfo.empty.start, bootinfo.empty.end);
        }

        // Delete the CNode setup by CAmkES; we're going to reuse the well-known
        // slot once it is empty (see MemoryManagerInterfaceThread::run below).
        unsafe {
            let path = &Camkes::top_level_path(MEMORY_RECV_CNODE);
            Camkes::delete_path(path).expect("recv_node");
        }
    }
}

type MemoryManagerResult = Result<Option<seL4_CPtr>, MemoryManagerError>;

struct MemoryInterfaceThread;
impl CamkesThreadInterface for MemoryInterfaceThread {
    fn run() {
        rpc_shared_recv_with_caps!(
            memory,
            MEMORY_RECV_CNODE,
            MEMORY_REQUEST_DATA_SIZE,
            MemoryManagerError::Success
        );
    }
}
impl MemoryInterfaceThread {
    fn dispatch(
        _client_badge: usize,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> MemoryManagerResult {
        let _cleanup = Camkes::cleanup_request_cap();
        let request = match postcard::from_bytes::<MemoryManagerRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(MemoryManagerError::DeserializeFailed),
        };
        match request {
            MemoryManagerRequest::Alloc {
                mut bundle,
                lifetime,
            } => Self::alloc_request(bundle.to_mut(), lifetime),
            MemoryManagerRequest::Free(mut bundle) => Self::free_request(bundle.to_mut()),
            MemoryManagerRequest::Stats => Self::stats_request(reply_buffer),

            MemoryManagerRequest::Debug => Self::debug_request(),
            MemoryManagerRequest::Capscan => Self::capscan_request(),
        }
    }

    fn alloc_request(bundle: &mut ObjDescBundle, lifetime: MemoryLifetime) -> MemoryManagerResult {
        // NB: make sure noone clobbers the setup done in memory__init;
        // and clear any capability the path points to when dropped, for next request
        let recv_path = unsafe { CAMKES.get_owned_current_recv_path() };
        // We must have a CNode for returning allocated objects.
        Camkes::debug_assert_slot_cnode("alloc_request", &recv_path);

        bundle.cnode = recv_path.1;
        // NB: bundle.depth should reflect the received cnode
        cantrip_memory().alloc(bundle, lifetime).map(|_| None)
    }

    fn free_request(bundle: &mut ObjDescBundle) -> MemoryManagerResult {
        // NB: make sure noone clobbers the setup done in memory__init;
        // and clear any capability the path points to when dropped, for next request
        let recv_path = unsafe { CAMKES.get_owned_current_recv_path() };
        // We must have a CNode for returning allocated objects.
        Camkes::debug_assert_slot_cnode("free_request", &recv_path);

        bundle.cnode = recv_path.1;
        // NB: bundle.depth should reflect the received cnode
        cantrip_memory().free(bundle).map(|_| None)
    }

    fn stats_request(reply_buffer: &mut [u8]) -> MemoryManagerResult {
        let recv_path = unsafe { CAMKES.get_current_recv_path() };
        // NB: make sure noone clobbers the setup done in memory__init
        unsafe {
            CAMKES.assert_recv_path();
        }
        // Verify no cap was received
        Camkes::debug_assert_slot_empty("stats_request", &recv_path);

        let stats = cantrip_memory().stats()?;
        let _ = postcard::to_slice(&StatsResponse { value: stats }, reply_buffer)
            .or(Err(MemoryManagerError::SerializeFailed))?;
        Ok(None)
    }

    fn debug_request() -> MemoryManagerResult {
        let recv_path = unsafe { CAMKES.get_current_recv_path() };
        // NB: make sure noone clobbers the setup done in memory__init
        unsafe {
            CAMKES.assert_recv_path();
        }
        Camkes::debug_assert_slot_empty("debug_request", &recv_path);

        cantrip_memory().debug().map(|_| None)
    }

    fn capscan_request() -> MemoryManagerResult {
        let recv_path = unsafe { CAMKES.get_current_recv_path() };
        // NB: make sure noone clobbers the setup done in memory__init
        unsafe {
            CAMKES.assert_recv_path();
        }
        Camkes::debug_assert_slot_empty("capscan_request", &recv_path);

        let _ = Camkes::capscan();
        Ok(None)
    }
}
