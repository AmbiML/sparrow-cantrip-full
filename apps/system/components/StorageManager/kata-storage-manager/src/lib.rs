//! Cantrip OS storage management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::String;
use cantrip_security_common::*;
use cantrip_storage_interface::{KeyValueData, KEY_VALUE_DATA_SIZE};
use cantrip_storage_interface::StorageError;
use cantrip_storage_interface::StorageManagerInterface;
use log::trace;
use postcard;

// NB: CANTRIP_STORAGE cannot be used before setup is completed with a call to init()
#[cfg(not(test))]
pub static mut CANTRIP_STORAGE: CantripStorageManager = CantripStorageManager{};

// CantripStorageManager bundles an instance of the StorageManager that operates
// on CantripOS interfaces. There is a two-step dance to setup an instance because
// we want CANTRIP_STORAGE static and there is no const Box::new variant.
pub struct CantripStorageManager;
impl StorageManagerInterface for CantripStorageManager {
    fn read(&self, bundle_id: &str, key: &str) -> Result<KeyValueData, StorageError> {
        trace!("read bundle_id:{} key:{}", bundle_id, key);

        fn serialize_failure(e: postcard::Error) -> StorageError {
            trace!("read: serialize failure {:?}", e);
            StorageError::SerializeFailed
        }

        // Send request to Security Core via SecurityCoordinator
        let mut request = [0u8; SECURITY_REQUEST_DATA_SIZE];
        let _ = postcard::to_slice(
            &ReadKeyRequest {
                bundle_id: String::from(bundle_id),
                key: String::from(key),
            },
            &mut request[..],
        )
        .map_err(serialize_failure)?;
        let result = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
        match cantrip_security_request(SecurityRequest::SrReadKey, &request, result) {
            SecurityRequestError::SreSuccess => {
                let mut keyval = [0u8; KEY_VALUE_DATA_SIZE];
                keyval.copy_from_slice(&result[..KEY_VALUE_DATA_SIZE]);
                Ok(keyval)
            }
            e => Err(map_security_request_error(e, StorageError::ReadFailed)),
        }
    }
    fn write(&self, bundle_id: &str, key: &str, value: &[u8]) -> Result<(), StorageError> {
        trace!(
            "write bundle_id:{} key:{} value:{:?}",
            bundle_id,
            key,
            value
        );

        fn serialize_failure(e: postcard::Error) -> StorageError {
            trace!("write: serialize failure {:?}", e);
            StorageError::SerializeFailed
        }

        // Send request to Security Core via SecurityCoordinator
        let mut request = [0u8; SECURITY_REQUEST_DATA_SIZE];
        let _ = postcard::to_slice(
            &WriteKeyRequest {
                bundle_id: String::from(bundle_id),
                key: String::from(key),
                value: value,
            },
            &mut request[..],
        )
        .map_err(serialize_failure)?;
        let result = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
        match cantrip_security_request(SecurityRequest::SrWriteKey, &request, result) {
            SecurityRequestError::SreSuccess => Ok(()),
            e => Err(map_security_request_error(e, StorageError::WriteFailed)),
        }
    }
    fn delete(&self, bundle_id: &str, key: &str) -> Result<(), StorageError> {
        trace!("delete bundle_id:{} key:{}", bundle_id, key);

        fn serialize_failure(e: postcard::Error) -> StorageError {
            trace!("delete: serialize failure {:?}", e);
            StorageError::SerializeFailed
        }

        // Send request to Security Core via SecurityCoordinator
        let mut request = [0u8; SECURITY_REQUEST_DATA_SIZE];
        let _ = postcard::to_slice(
            &DeleteKeyRequest {
                bundle_id: String::from(bundle_id),
                key: String::from(key),
            },
            &mut request[..],
        )
        .map_err(serialize_failure)?;
        let result = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
        match cantrip_security_request(SecurityRequest::SrDeleteKey, &request, result) {
            SecurityRequestError::SreSuccess => Ok(()),
            e => Err(map_security_request_error(e, StorageError::DeleteFailed)),
        }
    }
}

// Maps a SecuritRequestError to a StorageError.
fn map_security_request_error(sre: SecurityRequestError, def: StorageError) -> StorageError {
    match sre {
        SecurityRequestError::SreSuccess => StorageError::Success,
        SecurityRequestError::SreBundleNotFound => StorageError::BundleNotFound,
        SecurityRequestError::SreKeyNotFound => StorageError::KeyNotFound,
        _ => def,
    }
}
