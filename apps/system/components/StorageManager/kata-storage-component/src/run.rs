//! Cantrip OS StorageManager component support.

// Code here binds the camkes component to the rust code.
#![no_std]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;
use core::slice;
use cstr_core::CStr;
use cantrip_os_common::allocator;
use cantrip_os_common::logger::CantripLogger;
use cantrip_storage_interface::KeyValueData;
use cantrip_storage_interface::StorageManagerError;
use cantrip_storage_interface::StorageManagerInterface;
use cantrip_storage_manager::CANTRIP_STORAGE;
use log::trace;

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    // NB: set to max; the LoggerInterface will filter
    log::set_max_level(log::LevelFilter::Trace);

    // TODO(sleffler): temp until we integrate with seL4
    static mut HEAP_MEMORY: [u8; 8 * 1024] = [0; 8 * 1024];
    allocator::ALLOCATOR.init(HEAP_MEMORY.as_mut_ptr() as usize, HEAP_MEMORY.len());
    trace!(
        "setup heap: start_addr {:p} size {}",
        HEAP_MEMORY.as_ptr(),
        HEAP_MEMORY.len()
    );
}

// StorageInterface glue stubs.
#[no_mangle]
pub unsafe extern "C" fn storage_read(
    c_key: *const cstr_core::c_char,
    c_raw_value: *mut KeyValueData,
) -> StorageManagerError {
    match CStr::from_ptr(c_key).to_str() {
        Ok(key) => {
            // TODO(sleffler): de-badge reply cap to get bundle_id
            match CANTRIP_STORAGE.read("fubar", key) {
                Ok(value) => {
                    // NB: no serialization, returns raw data
                    (*c_raw_value).copy_from_slice(&value);
                    StorageManagerError::SmeSuccess
                }
                Err(e) => StorageManagerError::from(e),
            }
        }
        Err(_) => StorageManagerError::SmeKeyInvalid,
    }
}

#[no_mangle]
pub unsafe extern "C" fn storage_write(
    c_key: *const cstr_core::c_char,
    c_raw_value_len: usize,
    c_raw_value: *const u8,
) -> StorageManagerError {
    match CStr::from_ptr(c_key).to_str() {
        Ok(key) => {
            // TODO(sleffler): de-badge reply cap to get bundle_id
            CANTRIP_STORAGE.write(
                "fubar",
                key,
                slice::from_raw_parts(c_raw_value, c_raw_value_len),
            )
            .into()
        }
        Err(_) => StorageManagerError::SmeKeyInvalid,
    }
}

#[no_mangle]
pub unsafe extern "C" fn storage_delete(
    c_key: *const cstr_core::c_char
) -> StorageManagerError {
    match CStr::from_ptr(c_key).to_str() {
        Ok(key) => {
            // TODO(sleffler): de-badge reply cap to get bundle_id
            CANTRIP_STORAGE.delete("fubar", key).into()
        }
        Err(_) => StorageManagerError::SmeKeyInvalid,
    }
}
