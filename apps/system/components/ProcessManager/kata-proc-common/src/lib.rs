//! Cantrip OS process management support

#![cfg_attr(not(test), no_std)]
#![feature(array_methods)]

use bare_io::{Cursor, Write};
use core::convert::TryFrom;
use core::ops::{Index, IndexMut};
use core::str;
use smallstr::SmallString;
use smallvec::SmallVec;

// NB: struct's marked repr(C) are processed by cbindgen to get a .h file
//   used in camkes C interfaces.

// Bundle capacity before spillover to the heap.
pub const DEFAULT_BUNDLES_CAPACITY: usize = 10;

// BundleId capcity before spillover to the heap.
pub const DEFAULT_BUNDLE_ID_CAPACITY: usize = 64;

// BundleId encapsulates the pathname used to identify a Bundle (see the
// Cantrip OS design doc). BundleId's are used internally and exported through
// the BundleIdArray returned by get_running_bundles (TBD: maybe switch to
// String to reduce exposing internal details).
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct BundleId {
    pub id: SmallString<[u8; DEFAULT_BUNDLE_ID_CAPACITY]>,
}
impl BundleId {
    pub fn new() -> Self {
        BundleId {
            id: SmallString::with_capacity(DEFAULT_BUNDLE_ID_CAPACITY),
        }
    }
    pub fn len(&self) -> usize {
        self.id.len()
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.id.as_str().as_bytes()
    }
    pub fn as_str(&self) -> &str {
        self.id.as_str()
    }
    pub fn from_str(s: &str) -> BundleId {
        BundleId {
            id: SmallString::from_str(s),
        }
    }
}

// BundleIdArray is the collection of BundleId's returned by
// get_running_bundles (TBD: maybe switch to ArrayVec since we know
// the vector size at the construction time).
#[derive(Debug)]
pub struct BundleIdArray {
    pub ids: SmallVec<[BundleId; DEFAULT_BUNDLES_CAPACITY]>,
}
impl BundleIdArray {
    pub fn new() -> Self {
        BundleIdArray {
            ids: SmallVec::<[BundleId; DEFAULT_BUNDLES_CAPACITY]>::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.ids.len()
    }
    pub fn push(&mut self, id: &BundleId) {
        // NB: must manually copy; there is no Copy trait
        self.ids.push(BundleId::from_str(id.as_str()));
    }
    pub fn pop(&mut self) -> Option<BundleId> {
        self.ids.pop()
    }
    pub fn find(&self, s: &str) -> bool {
        let id = BundleId::from_str(s);
        self.ids.as_slice().iter().find(|&x| *x == id).is_some()
    }
}
impl Index<usize> for BundleIdArray {
    type Output = BundleId;
    fn index(&self, index: usize) -> &Self::Output {
        &self.ids[index]
    }
}
impl IndexMut<usize> for BundleIdArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.ids[index]
    }
}

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
        let bundle_count = [u8::try_from(bundles.len()).map_err(|_| bare_io::ErrorKind::InvalidData)?];
        result.write(&bundle_count[..])?; // # bundles
        for bid in bundles.ids.as_slice().iter() {
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
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Bundle {
    pub data: [u8; 128], // TODO(sleffler): placeholder
}
impl Bundle {
    pub fn new() -> Self {
        Bundle { data: [0u8; 128] }
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
    BundleNotFound,
    BundleFound,
    NoSpace,
    // Generic errors for interface failures.
    InstallFailed,
    UninstallFailed,
    StartFailed,
    StopFailed,
}

// Interface to underlying facilities (StorageManager, seL4); also
// used to inject fakes for unit tests.
pub trait ProcessManagerInterface {
    fn install(&self, bundle: &Bundle) -> Result<(), ProcessManagerError>;
    fn uninstall(&self, bundle: &Bundle) -> Result<(), ProcessManagerError>;
    fn start(&self, bundle: &Bundle) -> Result<(), ProcessManagerError>;
    fn stop(&self, bundle: &Bundle) -> Result<(), ProcessManagerError>;
}

// NB: bundle_id comes across the C interface as *const cstr_core::c_char
// and is converted to a &str using CStr::from_ptr().to_str().

pub trait PackageManagementInterface {
    fn install(&mut self, bundle_id: &str, bundle: &Bundle) -> Result<(), ProcessManagerError>;
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
    fn test_bundle_id_basics() {
        let bundle_id = BundleId::new();
        assert_eq!(bundle_id.len(), 0);
        assert_eq!(bundle_id.id.inline_size(), DEFAULT_BUNDLE_ID_CAPACITY);

        // Check str conversion.
        assert_eq!(BundleId::from_str("hello").as_str(), "hello");
    }

    #[test]
    fn test_bundle_id_array_basics() {
        let mut bid_array = BundleIdArray::new();

        // 1-element array.
        assert_eq!(bid_array.len(), 0);
        let bid = BundleId::from_str("hello");
        bid_array.push(&bid);
        assert_eq!(bid_array.len(), 1);
        assert_eq!(bid_array.find("foo"), false);
        assert_eq!(bid_array.find("hello"), true);
        assert_eq!(bid_array[0], bid);
        assert_eq!(bid_array.pop(), Some(bid));
        assert_eq!(bid_array.len(), 0);
        assert_eq!(bid_array.find("hello"), false);

        // Multiple entries.
        bid_array.push(&BundleId::from_str("zero"));
        bid_array.push(&BundleId::from_str("one"));
        bid_array.push(&BundleId::from_str("two"));
        assert_eq!(bid_array.len(), 3);
        assert_eq!(bid_array[1], BundleId::from_str("one"));
        bid_array[2] = BundleId::from_str("three");
        assert_eq!(bid_array.find("three"), true);
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
        bid_array.push(&BundleId::from_str("zero"));
        bid_array.push(&BundleId::from_str("one"));
        bid_array.push(&BundleId::from_str("two"));

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
        bid_array.push(&BundleId::from_str("zero"));
        bid_array.push(&BundleId::from_str("one"));
        bid_array.push(&BundleId::from_str("two"));

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
            bid_array.push(&BundleId::from_str(&bid.to_string()));
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
        bid_array.push(&BundleId::from_str(&"0123456789".repeat(26)));
        assert!(RawBundleIdData::new().pack_bundles(&bid_array).is_err());
    }
}
