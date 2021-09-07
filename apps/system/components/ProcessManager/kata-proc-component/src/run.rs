//! Cantrip OS ProcessManager component support.

// Code here binds the camkes component to the rust code.
#![no_std]

use cstr_core::CStr;
use cantrip_allocator;
use cantrip_logger::CantripLogger;
extern crate cantrip_panic;
use cantrip_proc_common::*;
use cantrip_proc_manager::CANTRIP_PROC;
use log::trace;
use postcard;

#[no_mangle]
pub extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    // NB: set to max; the LoggerInterface will filter
    log::set_max_level(log::LevelFilter::Trace);

    // TODO(sleffler): temp until we integrate with seL4
    static mut HEAP_MEMORY: [u8; 16 * 1024] = [0; 16 * 1024];
    unsafe {
        cantrip_allocator::ALLOCATOR.init(HEAP_MEMORY.as_mut_ptr() as usize, HEAP_MEMORY.len());
        trace!(
            "setup heap: start_addr {:p} size {}",
            HEAP_MEMORY.as_ptr(),
            HEAP_MEMORY.len()
        );
    }

    // Complete CANTRIP_PROC setup. This is as early as we can do it given that
    // it needs the GlobalAllocator.
    unsafe {
        CANTRIP_PROC.init();
        trace!(
            "ProcessManager has capacity for {} bundles",
            CANTRIP_PROC.capacity()
        );
    }
}

#[no_mangle]
pub extern "C" fn pkg_mgmt__init() {
    // Setup the userland address spaces, lifecycles, and system introspection
    // for third-party applications.
    trace!("init");
}

// PackageManagerInterface glue stubs.
#[no_mangle]
pub extern "C" fn pkg_mgmt_install(
    c_pkg_buffer_sz: usize,
    c_pkg_buffer: *const u8,
    c_raw_data: *mut RawBundleIdData,
) -> ProcessManagerError {
    unsafe {
        match CANTRIP_PROC.install(c_pkg_buffer, c_pkg_buffer_sz) {
            Ok(bundle_id) => match postcard::to_slice(&bundle_id, &mut (*c_raw_data)[..]) {
                Ok(_) => ProcessManagerError::Success,
                Err(e) => {
                    trace!("install failed: serialize {:?}", e);
                    ProcessManagerError::BundleDataInvalid
                }
            },
            Err(status) => {
                trace!("install failed: {:?}", status);
                status
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn pkg_mgmt_uninstall(c_bundle_id: *const cstr_core::c_char) -> ProcessManagerError {
    unsafe {
        match CStr::from_ptr(c_bundle_id).to_str() {
            Ok(bundle_id) => match CANTRIP_PROC.uninstall(bundle_id) {
                Ok(_) => ProcessManagerError::Success,
                Err(e) => e,
            },
            Err(_) => ProcessManagerError::BundleIdInvalid,
        }
    }
}

// ProcessControlInterface glue stubs.
#[no_mangle]
pub extern "C" fn proc_ctrl_start(c_bundle_id: *const cstr_core::c_char) -> ProcessManagerError {
    unsafe {
        match CStr::from_ptr(c_bundle_id).to_str() {
            Ok(bundle_id) => match CANTRIP_PROC.start(bundle_id) {
                Ok(_) => ProcessManagerError::Success,
                Err(e) => e,
            },
            Err(_) => ProcessManagerError::BundleIdInvalid,
        }
    }
}

#[no_mangle]
pub extern "C" fn proc_ctrl_stop(bundle_id: *const cstr_core::c_char) -> ProcessManagerError {
    unsafe {
        match CStr::from_ptr(bundle_id).to_str() {
            Ok(str) => match CANTRIP_PROC.stop(str) {
                Ok(_) => ProcessManagerError::Success,
                Err(e) => e,
            },
            Err(_) => ProcessManagerError::BundleIdInvalid,
        }
    }
}

#[no_mangle]
pub extern "C" fn proc_ctrl_get_running_bundles(
    c_raw_data: *mut RawBundleIdData,
) -> ProcessManagerError {
    unsafe {
        match CANTRIP_PROC.get_running_bundles() {
            Ok(bundles) => {
                // Serialize the bundle_id's in the result buffer. If we
                // overflow the buffer, BundleDataInvalid is returned and
                // the contents are undefined (postcard does not specify).
                match postcard::to_slice(&bundles, &mut (*c_raw_data)[..]) {
                    Ok(_) => ProcessManagerError::Success,
                    Err(e) => {
                        trace!("get_running_bundles failed: serialize {:?}", e);
                        ProcessManagerError::BundleDataInvalid
                    }
                }
            }
            Err(status) => {
                trace!("get_running_bundles failed: {:?}", status);
                status
            }
        }
    }
}
