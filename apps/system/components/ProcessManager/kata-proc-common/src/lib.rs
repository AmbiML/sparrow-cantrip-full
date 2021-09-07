//! Cantrip OS process management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::str;

pub type BundleIdArray = Vec<String>;

// NB: struct's marked repr(C) are processed by cbindgen to get a .h file
//   used in camkes C interfaces.

// BundleId capcity before spillover to the heap.
// TODO(sleffler): hide this; it's part of the implementation
pub const DEFAULT_BUNDLE_ID_CAPACITY: usize = 64;

// Size of the data buffer used to pass BundleIdArray data between Rust <> C.
// The data structure size is bounded by the camkes ipc buffer (120 bytes!)
// and also by it being allocated on the stack of the rpc glue code.
// So we need to balance these against being able to return all values.
pub const RAW_BUNDLE_ID_DATA_SIZE: usize = 100;
pub type RawBundleIdData = [u8; RAW_BUNDLE_ID_DATA_SIZE];

// TODO(sleffler): fill-in
#[derive(Clone, Debug)]
pub struct Bundle {
    // Bundle id extracted from manifest
    pub app_id: String,
    pub data: [u8; 64], // TODO(sleffler): placeholder
}
impl Bundle {
    pub fn new() -> Self {
        Bundle {
            app_id: String::with_capacity(DEFAULT_BUNDLE_ID_CAPACITY),
            data: [0u8; 64],
        }
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProcessManagerError {
    Success = 0,
    BundleIdInvalid,
    BundleDataInvalid,
    PackageBufferLenInvalid,
    BundleNotFound,
    BundleFound,
    BundleRunning,
    NoSpace,
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
    ) -> Result<Bundle, ProcessManagerError>;
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
