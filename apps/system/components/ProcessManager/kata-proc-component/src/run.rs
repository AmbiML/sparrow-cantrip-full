//! Cantrip OS ProcessManager component support.

// Code here binds the camkes component to the rust code.
#![no_std]

use cstr_core::CStr;
extern crate cantrip_panic;
use cantrip_allocator;
use cantrip_logger::CantripLogger;
use cantrip_proc_common::*;
use cantrip_proc_manager::CANTRIP_PROC;
use log::trace;

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
    bundle_id: *const cstr_core::c_char,
    bundle: Bundle,
) -> ProcessManagerError {
    unsafe {
        match CStr::from_ptr(bundle_id).to_str() {
            Ok(str) => match CANTRIP_PROC.install(str, &bundle) {
                Ok(_) => ProcessManagerError::Success,
                Err(e) => e,
            },
            Err(_) => ProcessManagerError::BundleIdInvalid,
        }
    }
}

#[no_mangle]
pub extern "C" fn pkg_mgmt_uninstall(bundle_id: *const cstr_core::c_char) -> ProcessManagerError {
    unsafe {
        match CStr::from_ptr(bundle_id).to_str() {
            Ok(str) => match CANTRIP_PROC.uninstall(str) {
                Ok(_) => ProcessManagerError::Success,
                Err(e) => e,
            },
            Err(_) => ProcessManagerError::BundleIdInvalid,
        }
    }
}

// ProcessControlInterface glue stubs.
#[no_mangle]
pub extern "C" fn proc_ctrl_start(bundle_id: *const cstr_core::c_char) -> ProcessManagerError {
    unsafe {
        match CStr::from_ptr(bundle_id).to_str() {
            Ok(str) => match CANTRIP_PROC.start(str) {
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
pub extern "C" fn proc_ctrl_get_running_bundles(c_raw_data: *mut u8) -> ProcessManagerError {
    unsafe {
        match CANTRIP_PROC.get_running_bundles() {
            Ok(bundles) => {
                // Serialize the bundle_id's in the result buffer as a series
                // of <length><value> pairs. If we overflow the buffer, nothing
                // is returned (should signal overflow somehow).
                // TODO(sleffler): pass buffer size instead of assuming?
                match RawBundleIdData::from_raw(
                    &mut *(c_raw_data as *mut [u8; RAW_BUNDLE_ID_DATA_SIZE]),
                )
                .pack_bundles(&bundles)
                {
                    Ok(_) => ProcessManagerError::Success,
                    Err(_) => ProcessManagerError::BundleDataInvalid,
                }
            }
            Err(e) => e,
        }
    }
}
