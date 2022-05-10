//! Cantrip OS process management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::str;
use cstr_core::CString;
use cantrip_security_interface::SecurityRequestError;
use postcard;
use serde::{Deserialize, Serialize};

mod bundle_image;
pub use bundle_image::*;

pub type BundleIdArray = Vec<String>;

// Size of the data buffer used to pass a serialized BundleIdArray between Rust <> C.
// The data structure size is bounded by the camkes ipc buffer (120 bytes!)
// and also by it being allocated on the stack of the rpc glue code.
// So we need to balance these against being able to return all values.
pub const RAW_BUNDLE_ID_DATA_SIZE: usize = 100;
pub type RawBundleIdData = [u8; RAW_BUNDLE_ID_DATA_SIZE];

// BundleId capacity before spillover to the heap.
// TODO(sleffler): hide this; it's part of the implementation
pub const DEFAULT_BUNDLE_ID_CAPACITY: usize = 64;

mod ptr_helper {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<T, S>(ptr: &*const T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (*ptr as usize).serialize(serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<*const T, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(usize::deserialize(deserializer)? as *const T)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bundle {
    // NB: application & ML binaries use well-known paths relative to bundle_id
    // NB: ProcessManager owns loaded application's memory

    // Bundle id extracted from manifest
    pub app_id: String,

    // Raw memory address of loaded application
    #[serde(with = "ptr_helper")]
    pub app_memory_address: *const u8,

    // Size (bytes) of loaded application
    pub app_memory_size: u32,
}
impl Bundle {
    pub fn new() -> Self {
        Bundle {
            app_id: String::with_capacity(DEFAULT_BUNDLE_ID_CAPACITY),
            app_memory_address: 0 as *const u8,
            app_memory_size: 0u32,
        }
    }
}

// NB: struct's marked repr(C) are processed by cbindgen to get a .h file
//   used in camkes C interfaces.

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProcessManagerError {
    Success = 0,
    BundleIdInvalid,
    PackageBufferLenInvalid,
    BundleNotFound,
    BundleFound,
    BundleRunning,
    UnknownError,
    DeserializeError,
    SerializeError,
    // Generic errors, mostly for unit tests.
    InstallFailed,
    UninstallFailed,
    StartFailed,
    StopFailed,
}

// Interface to underlying facilities (StorageManager, seL4); also
// used to inject fakes for unit tests.
pub trait ProcessManagerInterface {
    fn install(
        &mut self,
        pkg_buffer: *const u8,
        pkg_buffer_size: u32,
    ) -> Result<String, ProcessManagerError>;
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError>;
    fn start(&mut self, bundle: &Bundle) -> Result<(), ProcessManagerError>;
    fn stop(&mut self, bundle: &Bundle) -> Result<(), ProcessManagerError>;
}

// NB: pkg contents are in-memory and (likely) page-aligned so data can be
// passed across the C interface w/o a copy.

// NB: bundle_id comes across the C interface as *const cstr_core::c_char
// and is converted to a &str using CStr::from_ptr().to_str().

pub trait PackageManagementInterface {
    fn install(
        &mut self,
        pkg_buffer: *const u8,
        pkg_buffer_len: usize,
    ) -> Result<String, ProcessManagerError>;
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError>;
}

pub trait ProcessControlInterface {
    fn start(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError>;
    fn stop(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError>;
    fn get_running_bundles(&self) -> Result<BundleIdArray, ProcessManagerError>;
}

impl From<postcard::Error> for ProcessManagerError {
    fn from(err: postcard::Error) -> ProcessManagerError {
        match err {
            postcard::Error::SerializeBufferFull
            | postcard::Error::SerializeSeqLengthUnknown
            | postcard::Error::SerdeSerCustom => ProcessManagerError::SerializeError,
            // NB: bit of a cheat; this lumps in *Implement*
            _ => ProcessManagerError::DeserializeError,
        }
    }
}

impl From<SecurityRequestError> for ProcessManagerError {
    fn from(err: SecurityRequestError) -> ProcessManagerError {
        match err {
            SecurityRequestError::SreSuccess => ProcessManagerError::Success,
            SecurityRequestError::SreBundleIdInvalid => ProcessManagerError::BundleIdInvalid,
            SecurityRequestError::SreBundleNotFound => ProcessManagerError::BundleNotFound,
            SecurityRequestError::SrePackageBufferLenInvalid => {
                ProcessManagerError::PackageBufferLenInvalid
            }
            SecurityRequestError::SreInstallFailed => ProcessManagerError::InstallFailed,
            SecurityRequestError::SreUninstallFailed => ProcessManagerError::UninstallFailed,
            // NB: other errors "cannot happen" so just return something unique
            _ => ProcessManagerError::UnknownError,
        }
    }
}

impl From<cstr_core::NulError> for ProcessManagerError {
    fn from(_err: cstr_core::NulError) -> ProcessManagerError {
        ProcessManagerError::BundleIdInvalid
    }
}

impl From<ProcessManagerError> for Result<(), ProcessManagerError> {
    fn from(err: ProcessManagerError) -> Result<(), ProcessManagerError> {
        if err == ProcessManagerError::Success {
            Ok(())
        } else {
            Err(err)
        }
    }
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_proc_ctrl_get_running_bundles() -> Result<BundleIdArray, ProcessManagerError> {
    extern "C" {
        fn proc_ctrl_get_running_bundles(c_raw_data: *mut u8) -> ProcessManagerError;
    }
    let raw_data = &mut [0u8; RAW_BUNDLE_ID_DATA_SIZE];
    match unsafe { proc_ctrl_get_running_bundles(raw_data as *mut _) } {
        ProcessManagerError::Success => {
            let bids = postcard::from_bytes::<BundleIdArray>(raw_data)?;
            Ok(bids)
        }
        status => Err(status),
    }
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_pkg_mgmt_install(pkg_buffer: &[u8]) -> Result<String, ProcessManagerError> {
    extern "C" {
        fn pkg_mgmt_install(
            c_pkg_buffer_size: usize,
            c_pkg_buffer: *const u8,
            c_raw_data: *mut u8,
        ) -> ProcessManagerError;
    }
    let raw_data = &mut [0u8; RAW_BUNDLE_ID_DATA_SIZE];
    match unsafe { pkg_mgmt_install(pkg_buffer.len(), pkg_buffer.as_ptr(), raw_data as *mut _) } {
        ProcessManagerError::Success => {
            let bundle_id = postcard::from_bytes::<String>(raw_data.as_ref())?;
            Ok(bundle_id)
        }
        status => Err(status),
    }
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_pkg_mgmt_uninstall(bundle_id: &str) -> Result<(), ProcessManagerError> {
    extern "C" {
        fn pkg_mgmt_uninstall(c_bundle_id: *const cstr_core::c_char) -> ProcessManagerError;
    }
    let cstr = CString::new(bundle_id)?;
    unsafe { pkg_mgmt_uninstall(cstr.as_ptr()) }.into()
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_proc_ctrl_start(bundle_id: &str) -> Result<(), ProcessManagerError> {
    extern "C" {
        fn proc_ctrl_start(c_bundle_id: *const cstr_core::c_char) -> ProcessManagerError;
    }
    let cstr = CString::new(bundle_id)?;
    unsafe { proc_ctrl_start(cstr.as_ptr()) }.into()
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_proc_ctrl_stop(bundle_id: &str) -> Result<(), ProcessManagerError> {
    extern "C" {
        fn proc_ctrl_stop(c_bundle_id: *const cstr_core::c_char) -> ProcessManagerError;
    }
    let cstr = CString::new(bundle_id)?;
    unsafe { proc_ctrl_stop(cstr.as_ptr()) }.into()
}

// TODO(sleffler): move out of interface?
#[cfg(test)]
mod tests {
    use super::*;
    use postcard;

    #[test]
    fn test_raw_bundle_id_data_empty() {
        let bid_array = BundleIdArray::new();
        // Marhshall/unmarshall empty bid_array.
        let mut raw_data = [0u8; RAW_BUNDLE_ID_DATA_SIZE];
        assert!(postcard::to_slice(&bid_array, &mut raw_data).is_ok());
        assert_eq!(
            postcard::from_bytes::<BundleIdArray>(raw_data.as_ref()).unwrap(),
            bid_array
        );
    }

    #[test]
    fn test_raw_bundle_id_data_simple() {
        let mut bid_array = BundleIdArray::new();
        bid_array.push(String::from("zero"));
        bid_array.push(String::from("one"));
        bid_array.push(String::from("two"));

        // Marhshall/unmarshall bid_array.
        let mut raw_data = [0u8; RAW_BUNDLE_ID_DATA_SIZE];
        assert!(postcard::to_slice(&bid_array, &mut raw_data).is_ok());
        assert_eq!(
            postcard::from_bytes::<BundleIdArray>(raw_data.as_ref()).unwrap(),
            bid_array
        );
    }

    #[test]
    fn test_raw_bundle_id_data_out_of_space() {
        // Marshall an array with >255 id's; serialize will fail because
        // there's not enough space.
        let mut bid_array = BundleIdArray::new();
        for bid in 0..256 {
            bid_array.push(bid.to_string());
        }
        let mut raw_data = [0u8; RAW_BUNDLE_ID_DATA_SIZE];
        assert!(postcard::to_slice(&bid_array, &mut raw_data).is_err());
    }

    #[test]
    fn test_raw_bundle_id_data_too_long() {
        // Marshall an id with length >255; serialize will fail because
        // there's not enough space.
        let mut bid_array = BundleIdArray::new();
        bid_array.push("0123456789".repeat(26));
        let mut raw_data = [0u8; RAW_BUNDLE_ID_DATA_SIZE];
        assert!(postcard::to_slice(&bid_array, &mut raw_data).is_err());
    }
}
