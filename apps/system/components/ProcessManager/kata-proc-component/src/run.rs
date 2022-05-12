//! Cantrip OS ProcessManager component support.

// Code here binds the camkes component to the rust code.
#![no_std]
#![allow(clippy::missing_safety_doc)]

use core::slice;
use cstr_core::CStr;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::allocator;
use cantrip_os_common::logger::CantripLogger;
use cantrip_os_common::slot_allocator;
use cantrip_os_common::sel4_sys;
use cantrip_proc_interface::*;
use cantrip_proc_manager::CANTRIP_PROC;
use log::{info, trace};

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_GetCapReceivePath;
use sel4_sys::seL4_SetCapReceivePath;
use sel4_sys::seL4_Word;
use sel4_sys::seL4_WordBits;

use slot_allocator::CANTRIP_CSPACE_SLOTS;

// TODO(sleffler): belongs in sel4-sys
#[allow(non_camel_case_types)]
type seL4_Path = (seL4_CPtr, seL4_CPtr, seL4_Word);

extern "C" {
    // Each CAmkES-generated CNode has a writable self-reference to itself in
    // the slot SELF_CNODE.
    static SELF_CNODE: seL4_CPtr;

    static SELF_CNODE_FIRST_SLOT: seL4_CPtr;
    static SELF_CNODE_LAST_SLOT: seL4_CPtr;
}

// TODO(sleffler): 0 is valid
static mut PKG_MGMT_RECV_SLOT: seL4_CPtr = 0;

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    // NB: set to max; the LoggerInterface will filter
    log::set_max_level(log::LevelFilter::Trace);

    static mut HEAP_MEMORY: [u8; 16 * 1024] = [0; 16 * 1024];
    allocator::ALLOCATOR.init(HEAP_MEMORY.as_mut_ptr() as usize, HEAP_MEMORY.len());
    trace!(
        "setup heap: start_addr {:p} size {}",
        HEAP_MEMORY.as_ptr(),
        HEAP_MEMORY.len()
    );

    // Complete CANTRIP_PROC setup. This is as early as we can do it given that
    // it needs the GlobalAllocator.
    CANTRIP_PROC.init();
    trace!("ProcessManager has capacity for {} bundles", CANTRIP_PROC.capacity());

    CANTRIP_CSPACE_SLOTS.init(
        /*first_slot=*/ SELF_CNODE_FIRST_SLOT,
        /*size=*/ SELF_CNODE_LAST_SLOT - SELF_CNODE_FIRST_SLOT
    );
    trace!("setup cspace slots: first slot {} free {}",
           CANTRIP_CSPACE_SLOTS.base_slot(),
           CANTRIP_CSPACE_SLOTS.free_slots());

    PKG_MGMT_RECV_SLOT = CANTRIP_CSPACE_SLOTS.alloc(1).unwrap();
}

fn debug_check_empty(tag: &str, path: &seL4_Path) {
    sel4_sys::debug_assert_slot_empty!(path.1,
        "{}: expected slot {:?} empty but has cap type {:?}",
        tag, path, sel4_sys::cap_identify(path.1));
}


fn init_recv_path(tag: &str, path: &seL4_Path) {
    unsafe { seL4_SetCapReceivePath(path.0, path.1, path.2); }
    info!("{}: cap receive path {:?}", tag, path);
    debug_check_empty("init_recv_path", path);
}

#[no_mangle]
pub unsafe extern "C" fn pkg_mgmt__init() {
    // Point the receive path to the well-known slot for receiving
    // CNode's from clients for pkg_mgmt requests.
    //
    // NB: this must be done here (rather than someplace like pre_init)
    // so it's in the context of the PackageManagementInterface thread
    // (so we write the correct ipc buffer).
    init_recv_path("pkg_mgmt",
                   &(SELF_CNODE, PKG_MGMT_RECV_SLOT, seL4_WordBits));
}

// Clears any capability the specified path points to.
fn clear_path(path: &seL4_Path) {
    assert!(unsafe { seL4_CNode_Delete(path.0, path.1, path.2 as u8) }.is_ok());
    debug_check_empty("clear_path", path);
}

// PackageManagerInterface glue stubs.
#[no_mangle]
pub unsafe extern "C" fn pkg_mgmt_install(
    c_request_len: u32,
    c_request: *const u8,
    c_raw_data: *mut RawBundleIdData,
) -> ProcessManagerError {
    let recv_path = seL4_GetCapReceivePath();
    // NB: make sure noone clobbers the setup done in pkg_mgmt__init
    assert_eq!(recv_path, (SELF_CNODE, PKG_MGMT_RECV_SLOT, seL4_WordBits));

    let request_slice = slice::from_raw_parts(c_request, c_request_len as usize);
    let ret_status = match postcard::from_bytes::<ObjDescBundle>(request_slice) {
        Ok(mut pkg_contents) => {
            sel4_sys::debug_assert_slot_cnode!(recv_path.1,
                "Expected cnode in slot {} but has cap type {:?}",
                recv_path.1, sel4_sys::cap_identify(recv_path.1));
            pkg_contents.cnode = recv_path.1;
            match CANTRIP_PROC.install(&pkg_contents) {
                Ok(bundle_id) => match postcard::to_slice(&bundle_id, &mut (*c_raw_data)[..]) {
                    Ok(_) => ProcessManagerError::Success,
                    Err(_) => ProcessManagerError::SerializeError,
                },
                Err(e) => e,
            }
        }
        Err(e) => e.into(),
    };
    clear_path(&recv_path);
    ret_status
}

fn check_pkg_mgmt_empty(tag: &str) -> seL4_Path {
    unsafe {
        let recv_path = seL4_GetCapReceivePath();
        // NB: make sure noone clobbers the setup done in pkg_mgmt__init
        assert_eq!(recv_path, (SELF_CNODE, PKG_MGMT_RECV_SLOT, seL4_WordBits));
        debug_check_empty(tag, &recv_path);
        recv_path
    }
}

#[no_mangle]
pub unsafe extern "C" fn pkg_mgmt_uninstall(
    c_bundle_id: *const cstr_core::c_char
) -> ProcessManagerError {
    let recv_path = check_pkg_mgmt_empty("pkg_mgmt_uninstall");
    let ret_status = match CStr::from_ptr(c_bundle_id).to_str() {
        Ok(bundle_id) => match CANTRIP_PROC.uninstall(bundle_id) {
            Ok(_) => ProcessManagerError::Success,
            Err(e) => e,
        },
        Err(_) => ProcessManagerError::BundleIdInvalid,
    };
    debug_check_empty("pkg_mgmt_uninstall", &recv_path);
    ret_status
}

// ProcessControlInterface glue stubs.
#[no_mangle]
pub unsafe extern "C" fn proc_ctrl_start(
    c_bundle_id: *const cstr_core::c_char
) -> ProcessManagerError {
    match CStr::from_ptr(c_bundle_id).to_str() {
        Ok(bundle_id) => match CANTRIP_PROC.start(bundle_id) {
            Ok(_) => ProcessManagerError::Success,
            Err(e) => e,
        },
        Err(_) => ProcessManagerError::BundleIdInvalid,
    }
}

#[no_mangle]
pub unsafe extern "C" fn proc_ctrl_stop(
    c_bundle_id: *const cstr_core::c_char
) -> ProcessManagerError {
    match CStr::from_ptr(c_bundle_id).to_str() {
        Ok(str) => match CANTRIP_PROC.stop(str) {
            Ok(_) => ProcessManagerError::Success,
            Err(e) => e,
        },
        Err(_) => ProcessManagerError::BundleIdInvalid,
    }
}

#[no_mangle]
pub unsafe extern "C" fn proc_ctrl_get_running_bundles(
    c_raw_data: *mut RawBundleIdData,
) -> ProcessManagerError {
    match CANTRIP_PROC.get_running_bundles() {
        Ok(bundles) => {
            // Serialize the bundle_id's in the result buffer. If we
            // overflow the buffer, an error is returned and the
            // contents are undefined (postcard does not specify).
            match postcard::to_slice(&bundles, &mut (*c_raw_data)[..]) {
                Ok(_) => ProcessManagerError::Success,
                Err(_) => ProcessManagerError::DeserializeError,
            }
        }
        Err(e) => e,
    }
}
