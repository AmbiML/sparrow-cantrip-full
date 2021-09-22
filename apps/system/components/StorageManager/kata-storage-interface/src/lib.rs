//! Cantrip OS storage management support

#![cfg_attr(not(test), no_std)]

use core::str;
use cstr_core::CString;

// TODO(sleffler): temp constraint on value part of key-value pairs
pub const KEY_VALUE_DATA_SIZE: usize = 100;
pub type KeyValueData = [u8; KEY_VALUE_DATA_SIZE];

// NB: struct's marked repr(C) are processed by cbindgen to get a .h file
//   used in camkes C interfaces.

#[derive(Debug, Eq, PartialEq)]
pub enum StorageError {
    Success = 0,
    BundleNotFound,
    KeyNotFound,
    KeyInvalid,
    ValueInvalid,
    SerializeFailed,
    // Generic errors.
    ReadFailed,
    WriteFailed,
    DeleteFailed,
}

pub trait StorageManagerInterface {
    fn read(&self, bundle_id: &str, key: &str) -> Result<KeyValueData, StorageError>;
    fn write(&self, bundle_id: &str, key: &str, value: &[u8]) -> Result<(), StorageError>;
    fn delete(&self, bundle_id: &str, key: &str) -> Result<(), StorageError>;
}

// Public version of StorageError presented over rpc interface.
// This is needed because the enum is exported to C users and needs to
// be unique from other enum's.
// TODO(sleffler): switch to single generic error space ala absl::StatusCode
#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum StorageManagerError {
    SmeSuccess = 0,
    SmeBundleIdInvalid,
    SmeBundleNotFound,
    SmeKeyNotFound,
    SmeValueInvalid,
    SmeKeyInvalid,
    // Generic errors.
    SmeReadFailed,
    SmeWriteFailed,
    SmeDeleteFailed,
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_storage_delete(key: &str) -> Result<(), StorageManagerError> {
    // NB: this assumes the StorageManager component is named "storage".
    extern "C" {
        pub fn storage_delete(
            c_key: *const cstr_core::c_char
        ) -> StorageManagerError;
    }
    let cstr = CString::new(key)
        .map_err(|_| StorageManagerError::SmeKeyInvalid)?;
    match unsafe { storage_delete(cstr.as_ptr()) } {
        StorageManagerError::SmeSuccess => Ok(()),
        status => Err(status),
    }
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_storage_read(key: &str) -> Result<KeyValueData, StorageManagerError> {
    extern "C" {
        fn storage_read(
            c_key: *const cstr_core::c_char,
            c_raw_value: *mut KeyValueData,
        ) -> StorageManagerError;
    }
    let cstr = CString::new(key)
        .map_err(|_| StorageManagerError::SmeKeyInvalid)?;
    let value = &mut [0u8; KEY_VALUE_DATA_SIZE];
    match unsafe { storage_read(cstr.as_ptr(), value as *mut _) } {
        StorageManagerError::SmeSuccess => Ok(*value),
        status => Err(status),
    }
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_storage_write(key: &str, value: &[u8]) -> Result<(), StorageManagerError> {
    extern "C" {
        fn storage_write(
            c_key: *const cstr_core::c_char,
            c_raw_value_len: usize,
            c_raw_value: *const u8,
        ) -> StorageManagerError;
    }
    let cstr = CString::new(key)
        .map_err(|_| StorageManagerError::SmeKeyInvalid)?;
    match unsafe { storage_write(cstr.as_ptr(), value.len(), value.as_ptr()) } {
        StorageManagerError::SmeSuccess => Ok(()),
        status => Err(status),
    }
}
