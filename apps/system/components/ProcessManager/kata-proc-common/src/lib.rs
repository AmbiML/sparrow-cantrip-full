//! Cantrip OS process management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use bare_io::{Cursor, Write};
use core::convert::TryFrom;
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

#[repr(C)]
#[derive(Debug)]
pub struct RawBundleIdData {
    pub data: [u8; RAW_BUNDLE_ID_DATA_SIZE],
}
impl RawBundleIdData {
    pub fn new() -> Self {
        RawBundleIdData {
            data: [0u8; RAW_BUNDLE_ID_DATA_SIZE],
        }
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    // Construct from a raw buffer; this is used together with pack_bundles
    // to implement the producer side of the get_running_bundles() api (the
    // consumer side uses the iterator below).
    pub fn from_raw(raw: &mut [u8; RAW_BUNDLE_ID_DATA_SIZE]) -> &mut Self {
        unsafe { &mut *(raw as *mut _ as *mut Self) }
    }

    // Pack a collection of BundleId's into the buffer. A series of
    // <length><value> pairs are used with <length> a u8 so the max length
    // of a BundleId is 255.
    // TODO(sleffler): handle truncation better
    pub fn pack_bundles(&mut self, bundles: &BundleIdArray) -> bare_io::Result<()> {
        let mut result = Cursor::new(&mut self.data[..]);
        let bundle_count =
            [u8::try_from(bundles.len()).map_err(|_| bare_io::ErrorKind::InvalidData)?];
        result.write(&bundle_count[..])?; // # bundles
        for bid in bundles.as_slice().iter() {
            let bid_len = [u8::try_from(bid.len()).map_err(|_| bare_io::ErrorKind::InvalidData)?];
            result.write(&bid_len[..])?; // length
            result.write(bid.as_bytes())?; // value
        }
        Ok(())
    }

    // Returns an iterator over the packed BundleId's; useful on the
    // consumer side of the get_running_bundles() api.
    pub fn iter(&self) -> RawBundleIdDataIter {
        RawBundleIdDataIter {
            // The count of bundle id's is at the front.
            count: self.data[0],
            cur: &self.data[1..],
        }
    }
}

pub struct RawBundleIdDataIter<'a> {
    count: u8,     // Count of bundles.
    cur: &'a [u8], // Current slice in raw data buffer.
}
impl<'a> Iterator for RawBundleIdDataIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count > 0 {
            let bid_len = self.cur[0] as usize;
            let (bid, new_cur) = self.cur[1..].split_at(bid_len);
            let str = str::from_utf8(&bid).unwrap();
            self.cur = new_cur;
            self.count -= 1;
            Some(str)
        } else {
            None
        }
    }
}

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

    #[test]
    fn test_bundle_id_array_basics() {
        let mut bid_array = BundleIdArray::new();

        fn find_str(b: &BundleIdArray, id: &str) -> bool {
            b.as_slice().iter().find(|&x| *x == id).is_some()
        }

        // 1-element array.
        assert_eq!(bid_array.len(), 0);
        let bid = String::from("hello");
        bid_array.push(bid.clone());
        assert_eq!(bid_array.len(), 1);
        assert_eq!(find_str(&bid_array, "foo"), false);
        assert_eq!(find_str(&bid_array, "hello"), true);
        assert_eq!(bid_array[0], bid);
        assert_eq!(bid_array.pop(), Some(bid));
        assert_eq!(bid_array.len(), 0);
        assert_eq!(find_str(&bid_array, "hello"), false);

        // Multiple entries.
        bid_array.push(String::from("zero"));
        bid_array.push(String::from("one"));
        bid_array.push(String::from("two"));
        assert_eq!(bid_array.len(), 3);
        assert_eq!(bid_array[1], "one");
        bid_array[2] = String::from("three");
        assert_eq!(find_str(&bid_array, "three"), true);
    }

    #[test]
    fn test_raw_bundle_id_data_empty() {
        let bid_array = BundleIdArray::new();
        // Marhshall/unmarshall empty bid_array.
        let mut raw_data = RawBundleIdData::new();
        assert!(raw_data.pack_bundles(&bid_array).is_ok());
        assert_eq!(raw_data.iter().count(), 0);
    }

    #[test]
    fn test_raw_bundle_id_data_simple() {
        let mut bid_array = BundleIdArray::new();
        bid_array.push(String::from("zero"));
        bid_array.push(String::from("one"));
        bid_array.push(String::from("two"));

        // Marhshall/unmarshall bid_array.
        let mut raw_data = RawBundleIdData::new();
        assert!(raw_data.pack_bundles(&bid_array).is_ok());
        let mut iter = raw_data.iter();
        assert_eq!(iter.next(), Some("zero"));
        assert_eq!(iter.next(), Some("one"));
        assert_eq!(iter.next(), Some("two"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_raw_bundle_id_data_from_raw() {
        let mut bid_array = BundleIdArray::new();
        bid_array.push(String::from("zero"));
        bid_array.push(String::from("one"));
        bid_array.push(String::from("two"));

        // Marhshall bid_array.
        let mut raw_buf = [0u8; RAW_BUNDLE_ID_DATA_SIZE];
        let raw_data = RawBundleIdData::from_raw(&mut raw_buf);
        assert!(raw_data.pack_bundles(&bid_array).is_ok());

        // Unmarshall bid_array.
        let mut iter = raw_data.iter();
        assert_eq!(iter.next(), Some("zero"));
        assert_eq!(iter.next(), Some("one"));
        assert_eq!(iter.next(), Some("two"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_raw_bundle_id_data_out_of_space() {
        // Marshall an array with >255 id's; pack_bundles will fail because
        // the bundle count does not fit in a u8.
        // NB: this exceeds the array capacity so will spill to the heap;
        // that's ok for testing
        let mut bid_array = BundleIdArray::new();
        for bid in 0..256 {
            bid_array.push(bid.to_string());
        }
        assert!(RawBundleIdData::new().pack_bundles(&bid_array).is_err());
    }

    #[test]
    fn test_raw_bundle_id_data_too_long() {
        // Marshall an id with length >255; pack_bundles will fail because
        // the bundle id length does not fit in a u8.
        // NB: this exceeds the string capacity so will spill to the heap;
        // that's ok for testing
        let mut bid_array = BundleIdArray::new();
        bid_array.push("0123456789".repeat(26));
        assert!(RawBundleIdData::new().pack_bundles(&bid_array).is_err());
    }
}
