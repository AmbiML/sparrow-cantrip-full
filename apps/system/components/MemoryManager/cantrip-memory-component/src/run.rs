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
#![no_std]
#![allow(clippy::missing_safety_doc)]

use cantrip_memory_interface::MemoryManagerError;
use cantrip_memory_interface::MemoryManagerInterface;
use cantrip_memory_interface::MemoryManagerRequest;
use cantrip_memory_interface::MemoryResponseData;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_memory_interface::StatsResponse;
use cantrip_memory_manager::CantripMemoryManager;
use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::sel4_sys;
use core::ops::Range;
use core::slice;
use log::info;

use sel4_sys::seL4_BootInfo;
use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;

static mut CAMKES: Camkes = Camkes::new("MemoryManager");

// NB: CANTRIP_MEMORY cannot be used before setup is completed with a call to init()
static mut CANTRIP_MEMORY: CantripMemoryManager = CantripMemoryManager::empty();

extern "C" {
    // Each CAmkES-component has a CNode setup at a well-known top-level slot.
    // We re-use that slot to receive CNode caps passed with alloc & free requests.
    static MEMORY_RECV_CNODE: seL4_CPtr;
}

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    // NB: set to max; the LoggerInterface will filter
    CAMKES.init_logger(log::LevelFilter::Trace);

    static mut HEAP_MEMORY: [u8; 8 * 1024] = [0; 8 * 1024];
    CAMKES.init_allocator(&mut HEAP_MEMORY);

    extern "C" {
        fn sel4runtime_bootinfo() -> *const seL4_BootInfo;
    }
    // The MemoryManager component is labeled to receive BootInfo); use
    // it to complete initialization of the MemoryManager interface.
    let bootinfo = &*sel4runtime_bootinfo();
    CANTRIP_MEMORY.init(
        /*slots=*/
        Range::<seL4_CPtr> {
            start: bootinfo.untyped.start,
            end: bootinfo.untyped.end,
        },
        /*untypeds=*/ bootinfo.untyped_descs(),
    );
    if let Ok(stats) = CANTRIP_MEMORY.stats() {
        info!(
            "Global memory: {} allocated {} free, reserved: {} kernel {} user",
            stats.allocated_bytes,
            stats.free_bytes,
            bootinfo.kernelReservedBytes,
            stats.overhead_bytes,
        );
    }

    CAMKES.init_slot_allocator(bootinfo.empty.start, bootinfo.empty.end);

    // Delete the CAmkES-setup CNode; we're going to reuse the
    // well-known slot once it is empty (see memory__init below).
    let path = Camkes::top_level_path(MEMORY_RECV_CNODE);
    seL4_CNode_Delete(path.0, path.1, path.2 as u8).expect("recv_node");
}

#[no_mangle]
pub unsafe extern "C" fn memory__init() {
    // Point the receive path to the well-known slot that was emptied.
    // This will be used to receive CNode's from clients for alloc &
    // free requests.
    //
    // NB: this must be done here (rather than someplace like pre_init)
    // so it's in the context of the MemoryInterface thread (so we write
    // the correct ipc buffer).
    CAMKES.init_recv_path(&Camkes::top_level_path(MEMORY_RECV_CNODE));
}

// MemoryInterface glue stubs.

#[no_mangle]
pub unsafe extern "C" fn memory_request(
    c_request_buffer_len: u32,
    c_request_buffer: *const u8,
    c_reply_buffer: *mut MemoryResponseData,
) -> MemoryManagerError {
    let request_buffer = slice::from_raw_parts(c_request_buffer, c_request_buffer_len as usize);
    let request = match postcard::from_bytes::<MemoryManagerRequest>(request_buffer) {
        Ok(request) => request,
        Err(_) => return MemoryManagerError::MmeDeserializeFailed,
    };

    match request {
        MemoryManagerRequest::Alloc(mut bundle) => alloc_request(bundle.to_mut()),
        MemoryManagerRequest::Free(mut bundle) => free_request(bundle.to_mut()),
        MemoryManagerRequest::Stats => stats_request(&mut *c_reply_buffer),

        MemoryManagerRequest::Debug => debug_request(),
        MemoryManagerRequest::Capscan => capscan_request(),
    }
    .map_or_else(|e| e, |()| MemoryManagerError::MmeSuccess)
}

fn alloc_request(bundle: &mut ObjDescBundle) -> Result<(), MemoryManagerError> {
    // NB: make sure noone clobbers the setup done in memory__init;
    // and clear any capability the path points to when dropped, for next request
    let recv_path = unsafe { CAMKES.get_owned_current_recv_path() };
    // We must have a CNode for returning allocated objects.
    Camkes::debug_assert_slot_cnode("alloc_request", &recv_path);

    bundle.cnode = recv_path.1;
    // NB: bundle.depth should reflect the received cnode

    unsafe {
        CANTRIP_MEMORY.alloc(bundle)?;
    }
    Ok(())
}

fn free_request(bundle: &mut ObjDescBundle) -> Result<(), MemoryManagerError> {
    // NB: make sure noone clobbers the setup done in memory__init;
    // and clear any capability the path points to when dropped, for next request
    let recv_path = unsafe { CAMKES.get_owned_current_recv_path() };
    // We must have a CNode for returning allocated objects.
    Camkes::debug_assert_slot_cnode("free_request", &recv_path);

    bundle.cnode = recv_path.1;
    // NB: bundle.depth should reflect the received cnode
    unsafe {
        CANTRIP_MEMORY.free(bundle)?;
    }
    Ok(())
}

fn stats_request(reply_buffer: &mut MemoryResponseData) -> Result<(), MemoryManagerError> {
    let recv_path = unsafe { CAMKES.get_current_recv_path() };
    // NB: make sure noone clobbers the setup done in memory__init
    unsafe {
        CAMKES.assert_recv_path();
    }

    let stats = unsafe { CANTRIP_MEMORY.stats() }?;
    // Verify no cap was received
    Camkes::debug_assert_slot_empty("stats_request", &recv_path);
    let _ = postcard::to_slice(&StatsResponse { value: stats }, reply_buffer)
        .or(Err(MemoryManagerError::MmeSerializeFailed))?;
    Ok(())
}

fn debug_request() -> Result<(), MemoryManagerError> {
    let recv_path = unsafe { CAMKES.get_current_recv_path() };
    // NB: make sure noone clobbers the setup done in memory__init
    unsafe {
        CAMKES.assert_recv_path();
    }
    Camkes::debug_assert_slot_empty("debug_request", &recv_path);

    unsafe {
        CANTRIP_MEMORY.debug()?;
    }
    Ok(())
}

fn capscan_request() -> Result<(), MemoryManagerError> {
    let recv_path = unsafe { CAMKES.get_current_recv_path() };
    // NB: make sure noone clobbers the setup done in memory__init
    unsafe {
        CAMKES.assert_recv_path();
    }
    Camkes::debug_assert_slot_empty("capscan_request", &recv_path);

    let _ = Camkes::capscan();
    Ok(())
}
