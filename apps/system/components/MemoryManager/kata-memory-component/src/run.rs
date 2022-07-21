//! Cantrip OS MemoryManager component support.

// Code here binds the camkes component to the rust code.
#![no_std]
#![allow(clippy::missing_safety_doc)]

use core::ops::Range;
use core::slice;
use cantrip_memory_interface::MemoryManagerError;
use cantrip_memory_interface::MemoryManagerInterface;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_memory_interface::RawMemoryStatsData;
use cantrip_memory_manager::CantripMemoryManager;
use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::sel4_sys;
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
            "Global memory: {} allocated {} free {} reserved",
            stats.allocated_bytes, stats.free_bytes, stats.overhead_bytes,
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
pub unsafe extern "C" fn memory_alloc(
    c_raw_data_len: u32,
    c_raw_data: *const u8,
) -> MemoryManagerError {
    let recv_path = CAMKES.get_current_recv_path();
    // NB: make sure noone clobbers the setup done in memory__init
    CAMKES.assert_recv_path();

    let raw_slice = slice::from_raw_parts(c_raw_data, c_raw_data_len as usize);
    let ret_status = match postcard::from_bytes::<ObjDescBundle>(raw_slice) {
        Ok(mut bundle) => {
            // We must have a CNode for returning allocated objects.
            Camkes::debug_assert_slot_cnode("memory_alloc", &recv_path);

            bundle.cnode = recv_path.1;
            // NB: bundle.depth should reflect the received cnode
            CANTRIP_MEMORY.alloc(&bundle).into()
        }
        Err(_) => MemoryManagerError::MmeDeserializeFailed,
    };
    // NB: must clear ReceivePath for next request
    CAMKES.clear_recv_path();
    ret_status
}

#[no_mangle]
pub unsafe extern "C" fn memory_free(
    c_raw_data_len: u32,
    c_raw_data: *const u8,
) -> MemoryManagerError {
    let recv_path = CAMKES.get_current_recv_path();
    // NB: make sure noone clobbers the setup done in memory__init
    CAMKES.assert_recv_path();

    let raw_slice = slice::from_raw_parts(c_raw_data, c_raw_data_len as usize);
    let ret_status = match postcard::from_bytes::<ObjDescBundle>(raw_slice) {
        Ok(mut bundle) => {
            // We must have a CNode for returning allocated objects.
            Camkes::debug_assert_slot_cnode("memory_free", &recv_path);

            bundle.cnode = recv_path.1;
            // NB: bundle.depth should reflect the received cnode
            CANTRIP_MEMORY.free(&bundle).into()
        }
        Err(_) => MemoryManagerError::MmeDeserializeFailed,
    };
    // NB: must clear ReceivePath for next request
    CAMKES.clear_recv_path();
    ret_status
}

#[no_mangle]
pub unsafe extern "C" fn memory_stats(
    c_raw_resp_data: *mut RawMemoryStatsData,
) -> MemoryManagerError {
    let recv_path = CAMKES.get_current_recv_path();
    // NB: make sure noone clobbers the setup done in memory__init
    CAMKES.assert_recv_path();

    match CANTRIP_MEMORY.stats() {
        Ok(stats) => {
            // Verify no cap was received
            Camkes::debug_assert_slot_empty("memory_stats", &recv_path);

            match postcard::to_slice(&stats, &mut (*c_raw_resp_data)[..]) {
                Ok(_) => MemoryManagerError::MmeSuccess,
                Err(_) => MemoryManagerError::MmeSerializeFailed,
            }
        }
        Err(e) => e.into(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn memory_debug() -> MemoryManagerError {
    let recv_path = CAMKES.get_current_recv_path();
    // NB: make sure noone clobbers the setup done in memory__init
    CAMKES.assert_recv_path();
    Camkes::debug_assert_slot_empty("memory_debug", &recv_path);

    CANTRIP_MEMORY.debug().into()
}

#[no_mangle]
pub unsafe extern "C" fn memory_capscan() {
    let recv_path = CAMKES.get_current_recv_path();
    // NB: make sure noone clobbers the setup done in memory__init
    CAMKES.assert_recv_path();
    Camkes::debug_assert_slot_empty("memory_debug", &recv_path);

    let _ = Camkes::capscan();
}
