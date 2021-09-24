//! Cantrip OS storage management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::String;
use cantrip_security_interface::cantrip_security_request;
use cantrip_security_interface::DeleteKeyRequest;
use cantrip_security_interface::ReadKeyRequest;
use cantrip_security_interface::SecurityRequest;
use cantrip_security_interface::WriteKeyRequest;
use cantrip_security_interface::SECURITY_REPLY_DATA_SIZE;
use cantrip_security_interface::SECURITY_REQUEST_DATA_SIZE;
use cantrip_storage_interface::StorageError;
use cantrip_storage_interface::StorageManagerInterface;
use cantrip_storage_interface::{KeyValueData, KEY_VALUE_DATA_SIZE};
use log::trace;
use postcard;

// NB: CANTRIP_STORAGE cannot be used before setup is completed with a call to init()
#[cfg(not(test))]
pub static mut CANTRIP_STORAGE: CantripStorageManager = CantripStorageManager {};

// CantripStorageManager bundles an instance of the StorageManager that operates
// on CantripOS interfaces. There is a two-step dance to setup an instance because
// we want CANTRIP_STORAGE static and there is no const Box::new variant.
pub struct CantripStorageManager;
impl StorageManagerInterface for CantripStorageManager {
    fn read(&self, bundle_id: &str, key: &str) -> Result<KeyValueData, StorageError> {
        trace!("read bundle_id:{} key:{}", bundle_id, key);

        // Send request to Security Core via SecurityCoordinator
        let mut request = [0u8; SECURITY_REQUEST_DATA_SIZE];
        let _ = postcard::to_slice(
            &ReadKeyRequest {
                bundle_id: String::from(bundle_id),
                key: String::from(key),
            },
            &mut request[..],
        )?;
        let result = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
        let _ = cantrip_security_request(SecurityRequest::SrReadKey, &request, result)?;
        // NB: must copy into KeyValueData for now
        let mut keyval = [0u8; KEY_VALUE_DATA_SIZE];
        keyval.copy_from_slice(&result[..KEY_VALUE_DATA_SIZE]);
        Ok(keyval)
    }
    fn write(&self, bundle_id: &str, key: &str, value: &[u8]) -> Result<(), StorageError> {
        trace!(
            "write bundle_id:{} key:{} value:{:?}",
            bundle_id,
            key,
            value
        );

        // Send request to Security Core via SecurityCoordinator
        let mut request = [0u8; SECURITY_REQUEST_DATA_SIZE];
        let _ = postcard::to_slice(
            &WriteKeyRequest {
                bundle_id: String::from(bundle_id),
                key: String::from(key),
                value: value,
            },
            &mut request[..],
        )?;
        let result = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
        cantrip_security_request(SecurityRequest::SrWriteKey, &request, result)?;
        Ok(())
    }
    fn delete(&self, bundle_id: &str, key: &str) -> Result<(), StorageError> {
        trace!("delete bundle_id:{} key:{}", bundle_id, key);

        // Send request to Security Core via SecurityCoordinator
        let mut request = [0u8; SECURITY_REQUEST_DATA_SIZE];
        let _ = postcard::to_slice(
            &DeleteKeyRequest {
                bundle_id: String::from(bundle_id),
                key: String::from(key),
            },
            &mut request[..],
        )?;
        let result = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
        cantrip_security_request(SecurityRequest::SrDeleteKey, &request, result)?;
        Ok(())
    }
}
