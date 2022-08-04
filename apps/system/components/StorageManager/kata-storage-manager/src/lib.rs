//! Cantrip OS storage management support

#![cfg_attr(not(test), no_std)]

use cantrip_security_interface::cantrip_security_delete_key;
use cantrip_security_interface::cantrip_security_read_key;
use cantrip_security_interface::cantrip_security_write_key;
use cantrip_storage_interface::StorageError;
use cantrip_storage_interface::StorageManagerInterface;
use cantrip_storage_interface::{KeyValueData, KEY_VALUE_DATA_SIZE};
use log::trace;

#[cfg(not(test))]
pub static mut CANTRIP_STORAGE: CantripStorageManager = CantripStorageManager {};

pub struct CantripStorageManager;
impl StorageManagerInterface for CantripStorageManager {
    fn read(&self, bundle_id: &str, key: &str) -> Result<KeyValueData, StorageError> {
        trace!("read bundle_id:{} key:{}", bundle_id, key);

        // NB: must copy into KeyValueData for now
        let mut keyval = [0u8; KEY_VALUE_DATA_SIZE];
        Ok(cantrip_security_read_key(bundle_id, key, &mut keyval).map(|_| keyval)?)
    }
    fn write(&self, bundle_id: &str, key: &str, value: &[u8]) -> Result<(), StorageError> {
        trace!("write bundle_id:{} key:{} value:{:?}", bundle_id, key, value);

        Ok(cantrip_security_write_key(bundle_id, key, value)?)
    }
    fn delete(&self, bundle_id: &str, key: &str) -> Result<(), StorageError> {
        trace!("delete bundle_id:{} key:{}", bundle_id, key);

        Ok(cantrip_security_delete_key(bundle_id, key)?)
    }
}
