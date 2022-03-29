//! Cantrip OS MemoryManager component support.

// Code here binds the camkes component to the rust code.
#![no_std]

use core::ops::Range;
use core::slice;
extern crate cantrip_panic;
use cantrip_allocator;
use cantrip_logger::CantripLogger;
use cantrip_memory_interface::MemoryManagerError;
use cantrip_memory_interface::MemoryManagerInterface;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_memory_interface::RawMemoryStatsData;
use cantrip_memory_manager::CantripMemoryManager;
use cantrip_os_common::sel4_sys;
use log::{info, trace};
use sel4_sys::seL4_BootInfo;
use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_GetCapReceivePath;
use sel4_sys::seL4_SetCapReceivePath;
use sel4_sys::seL4_Word;
use sel4_sys::seL4_WordBits;

// NB: CANTRIP_MEMORY cannot be used before setup is completed with a call to init()
static mut CANTRIP_MEMORY: CantripMemoryManager = CantripMemoryManager::empty();

extern "C" {
    // Each CAmkES-generated CNode has a writable self-reference to itself in
    // the slot SELF_CNODE. This is used to pass CNode containers of dynamically
    // allocated objects between clients & the MemoryManager.
    static SELF_CNODE: seL4_CPtr;

    // Each CAmkES-component has a CNode setup at a well-known slot in SELF_CNODE.
    // We re-use that slot to receive CNode caps passed with alloc & free requests.
    static RECV_CNODE: seL4_CPtr;
}

#[no_mangle]
pub extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    // NB: set to max; the LoggerInterface will filter
    log::set_max_level(log::LevelFilter::Trace);

    // TODO(sleffler): temp until we integrate with seL4
    static mut HEAP_MEMORY: [u8; 8 * 1024] = [0; 8 * 1024];
    unsafe {
        cantrip_allocator::ALLOCATOR.init(HEAP_MEMORY.as_mut_ptr() as usize, HEAP_MEMORY.len());
        trace!(
            "setup heap: start_addr {:p} size {}",
            HEAP_MEMORY.as_ptr(),
            HEAP_MEMORY.len()
        );
    }

    extern "C" {
        fn sel4runtime_bootinfo() -> *const seL4_BootInfo;
    }
    unsafe {
        // The MemoryManager component is labeled to receive BootInfo); use
        // it to complete initialization of the MemoryManager interface.
        let bootinfo = &*sel4runtime_bootinfo();
        CANTRIP_MEMORY.init(
            /*slots=*/Range::<seL4_CPtr> {
                start: bootinfo.untyped.start,
                end: bootinfo.untyped.end
            },
            /*untypeds=*/ bootinfo.untyped_descs(),
        );
        if let Ok(stats) = CANTRIP_MEMORY.stats() {
            trace!("Global memory: {} allocated {} free",
                stats.allocated_bytes,
                stats.free_bytes,
            );
        }
    }
    unsafe {
        // Delete the CAmkES-setup CNode; we're going to reuse the
        // well-known slot once it is empty (see below).
        seL4_CNode_Delete(SELF_CNODE, RECV_CNODE, seL4_WordBits as u8)
            .expect("recv_node");
    }
}

#[no_mangle]
pub extern "C" fn memory__init() {
    unsafe {
        // Point the receive path to the well-known slot that was emptied.
        // This will be used to receive CNode's from clients for alloc &
        // free requests.
        //
        // NB: this must be done here (rather than someplace like pre_init)
        // so it's in the context of the MemoryInterface thread (so we write
        // the correct ipc buffer).
        seL4_SetCapReceivePath(SELF_CNODE, RECV_CNODE, seL4_WordBits);
        trace!("Cap receive path {}:{}:{}", SELF_CNODE, RECV_CNODE, seL4_WordBits);
    }
}

// MemoryInterface glue stubs.

// Clears any capability the specified path points to.
fn clear_path(&(root, index, depth): &(seL4_CPtr, seL4_CPtr, seL4_Word)) {
    // TODO(sleffler): assert since future receives are likely to fail?
    if let Err(e) = unsafe { seL4_CNode_Delete(root, index, depth as u8) } {
        // NB: no error is returned if the slot is empty.
        info!("Failed to clear receive path {:?}: {:?}",
              (root, index, depth), e);
    }
}

#[no_mangle]
pub extern "C" fn memory_alloc(
    c_raw_data_len: u32,
    c_raw_data: *const u8,
) -> MemoryManagerError {
    unsafe {
        let recv_path = seL4_GetCapReceivePath();
        // NB: make sure noone clobbers the setup done in memory__init
        assert_eq!(recv_path, (SELF_CNODE, RECV_CNODE, seL4_WordBits));

        let raw_slice = slice::from_raw_parts(c_raw_data, c_raw_data_len as usize);
        let ret_status = match postcard::from_bytes::<ObjDescBundle>(raw_slice) {
            Ok(mut bundle) => {
                // TODO(sleffler): verify we received a CNode in RECV_CNODE.
                bundle.cnode = recv_path.1;
                // NB: bundle.depth should reflect the received cnode
                CANTRIP_MEMORY.alloc(&bundle).into()
            }
            Err(_) => MemoryManagerError::MmeDeserializeFailed,
        };
        // NB: must clear ReceivePath for next request
        clear_path(&recv_path);
        ret_status
    }
}

#[no_mangle]
pub extern "C" fn memory_free(
    c_raw_data_len: u32,
    c_raw_data: *const u8,
) -> MemoryManagerError {
    unsafe {
        let recv_path = seL4_GetCapReceivePath();
        // NB: make sure noone clobbers the setup done in memory__init
        assert_eq!(recv_path, (SELF_CNODE, RECV_CNODE, seL4_WordBits));

        let raw_slice = slice::from_raw_parts(c_raw_data, c_raw_data_len as usize);
        let ret_status = match postcard::from_bytes::<ObjDescBundle>(raw_slice) {
            Ok(mut bundle) => {
                // TODO(sleffler): verify we received a CNode in RECV_CNODE.
                bundle.cnode = recv_path.1;
                // NB: bundle.depth should reflect the received cnode
                CANTRIP_MEMORY.free(&bundle).into()
            }
            Err(_) => MemoryManagerError::MmeDeserializeFailed,
        };
        // NB: must clear ReceivePath for next request
        clear_path(&recv_path);
        ret_status
    }
}

#[no_mangle]
pub extern "C" fn memory_stats(
    c_raw_resp_data: *mut RawMemoryStatsData,
) -> MemoryManagerError {
    unsafe {
        // TODO(sleffler): verify no cap was received
        match CANTRIP_MEMORY.stats() {
            Ok(stats) => {
                  match postcard::to_slice(&stats, &mut (*c_raw_resp_data)[..]) {
                      Ok(_) => MemoryManagerError::MmeSuccess,
                      Err(_) => MemoryManagerError::MmeSerializeFailed,
                  }
            }
            Err(e) => e.into(),
        }
    }
}
