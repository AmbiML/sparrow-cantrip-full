//! Cantrip OS ProcessManager component support.

// Code here binds the camkes component to the rust code.
#![no_std]
#![allow(clippy::missing_safety_doc)]

use core::slice;
use cstr_core::CStr;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::slot_allocator;
use cantrip_os_common::sel4_sys;
use cantrip_proc_interface::*;
use cantrip_proc_manager::CANTRIP_PROC;
use log::trace;

use sel4_sys::seL4_CPtr;

use slot_allocator::CANTRIP_CSPACE_SLOTS;

static mut CAMKES: Camkes = Camkes::new("ProcessManager");

// TODO(sleffler): 0 is valid
static mut PKG_MGMT_RECV_SLOT: seL4_CPtr = 0;

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static mut HEAP_MEMORY: [u8; 16 * 1024] = [0; 16 * 1024];
    CAMKES.pre_init(log::LevelFilter::Trace, &mut HEAP_MEMORY);

    // Complete CANTRIP_PROC setup now that Global allocator is setup.
    CANTRIP_PROC.init();
    trace!("ProcessManager has capacity for {} bundles", CANTRIP_PROC.capacity());

    PKG_MGMT_RECV_SLOT = CANTRIP_CSPACE_SLOTS.alloc(1).unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn pkg_mgmt__init() {
    // Point the receive path to the well-known slot for receiving
    // CNode's from clients for pkg_mgmt requests.
    //
    // NB: this must be done here (rather than someplace like pre_init)
    // so it's in the context of the PackageManagementInterface thread
    // (so we write the correct ipc buffer).
    CAMKES.init_recv_path(&Camkes::top_level_path(PKG_MGMT_RECV_SLOT));
}

// PackageManagerInterface glue stubs.
#[no_mangle]
pub unsafe extern "C" fn pkg_mgmt_install(
    c_request_len: u32,
    c_request: *const u8,
    c_raw_data: *mut RawBundleIdData,
) -> ProcessManagerError {
    let recv_path = CAMKES.get_current_recv_path();
    // NB: make sure noone clobbers the setup done in pkg_mgmt__init
    CAMKES.assert_recv_path();

    let request_slice = slice::from_raw_parts(c_request, c_request_len as usize);
    let ret_status = match postcard::from_bytes::<ObjDescBundle>(request_slice) {
        Ok(mut pkg_contents) => {
            Camkes::debug_assert_slot_cnode("pkg_mgmt_install", &recv_path);
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
    CAMKES.clear_recv_path();
    ret_status
}

#[no_mangle]
pub unsafe extern "C" fn pkg_mgmt_uninstall(
    c_bundle_id: *const cstr_core::c_char
) -> ProcessManagerError {
    let recv_path = CAMKES.get_current_recv_path();
    CAMKES.assert_recv_path();
    Camkes::debug_assert_slot_empty("pkg_mgmt_uninstall", &recv_path);
    let ret_status = match CStr::from_ptr(c_bundle_id).to_str() {
        Ok(bundle_id) => match CANTRIP_PROC.uninstall(bundle_id) {
            Ok(_) => ProcessManagerError::Success,
            Err(e) => e,
        },
        Err(_) => ProcessManagerError::BundleIdInvalid,
    };
    Camkes::debug_assert_slot_empty("pkg_mgmt_uninstall", &recv_path);
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

#[no_mangle]
pub unsafe extern "C" fn proc_ctrl_capscan() {
    let _ = Camkes::capscan();
}

#[no_mangle]
pub unsafe extern "C" fn proc_ctrl_capscan_bundle(
    c_bundle_id: *const cstr_core::c_char
) -> ProcessManagerError {
    match CStr::from_ptr(c_bundle_id).to_str() {
        Ok(str) => match CANTRIP_PROC.capscan(str) {
            Ok(_) => ProcessManagerError::Success,
            Err(e) => e,
        },
        Err(_) => ProcessManagerError::BundleIdInvalid,
    }
}
