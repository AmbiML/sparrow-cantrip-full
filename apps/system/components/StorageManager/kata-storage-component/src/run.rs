//! Cantrip OS StorageManager component support.

// Code here binds the camkes component to the rust code.
#![no_std]

extern crate alloc;
use core::slice;
use cstr_core::CStr;
extern crate cantrip_panic;
use cantrip_allocator;
use cantrip_logger::CantripLogger;
use cantrip_storage_interface::KeyValueData;
use cantrip_storage_interface::StorageError;
use cantrip_storage_interface::StorageManagerError;
use cantrip_storage_interface::StorageManagerInterface;
use cantrip_storage_manager::CANTRIP_STORAGE;
use log::trace;

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
}

fn map_storage_error(se: StorageError, def: StorageManagerError) -> StorageManagerError {
    match se {
        StorageError::Success => StorageManagerError::SmeSuccess,
        StorageError::BundleNotFound => StorageManagerError::SmeBundleNotFound,
        StorageError::KeyNotFound => StorageManagerError::SmeKeyNotFound,
        StorageError::KeyInvalid => StorageManagerError::SmeKeyInvalid,
        StorageError::ValueInvalid => StorageManagerError::SmeValueInvalid,
        _ => def,
    }
}

// StorageInterface glue stubs.
#[no_mangle]
pub extern "C" fn storage_read(
    c_key: *const cstr_core::c_char,
    c_raw_value: *mut KeyValueData,
) -> StorageManagerError {
    unsafe {
        match CStr::from_ptr(c_key).to_str() {
            Ok(key) => {
                // TODO(sleffler): de-badge reply cap to get bundle_id
                match CANTRIP_STORAGE.read("fubar", key) {
                    // NB: no serialization, returns raw data
                    Ok(value) => {
                        (*c_raw_value).copy_from_slice(&value);
                        StorageManagerError::SmeSuccess
                    }
                    Err(e) => map_storage_error(e, StorageManagerError::SmeReadFailed),
                }
            }
            Err(e) => {
                trace!("read: keyinvalid {:?}", e);
                StorageManagerError::SmeKeyInvalid
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn storage_write(
    c_key: *const cstr_core::c_char,
    c_raw_value_len: usize,
    c_raw_value: *const u8,
) -> StorageManagerError {
    unsafe {
        match CStr::from_ptr(c_key).to_str() {
            Ok(key) => {
                // TODO(sleffler): de-badge reply cap to get bundle_id
                match CANTRIP_STORAGE.write(
                    "fubar",
                    key,
                    slice::from_raw_parts(c_raw_value, c_raw_value_len),
                ) {
                    Ok(_) => StorageManagerError::SmeSuccess,
                    Err(e) => map_storage_error(e, StorageManagerError::SmeWriteFailed),
                }
            }
            Err(e) => {
                trace!("write: keyinvalid {:?}", e);
                StorageManagerError::SmeKeyInvalid
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn storage_delete(c_key: *const cstr_core::c_char) -> StorageManagerError {
    unsafe {
        match CStr::from_ptr(c_key).to_str() {
            Ok(key) => {
                // TODO(sleffler): de-badge reply cap to get bundle_id
                match CANTRIP_STORAGE.delete("fubar", key) {
                    Ok(_) => StorageManagerError::SmeSuccess,
                    Err(e) => map_storage_error(e, StorageManagerError::SmeDeleteFailed),
                }
            }
            Err(e) => {
                trace!("delete: keyinvalid {:?}", e);
                StorageManagerError::SmeKeyInvalid
            }
        }
    }
}
