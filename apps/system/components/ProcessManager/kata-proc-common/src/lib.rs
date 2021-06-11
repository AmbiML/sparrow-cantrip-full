//! Cantrip OS process management support

#![cfg_attr(not(test), no_std)]

use core::ops::{Index, IndexMut};

// NB: struct's marked repr(C) are processed by cbindgen to get a .h file
//   used in camkes interfaces.

// Max bundles that can be installed at one time.
pub const MAX_BUNDLES: usize = 10;
// Max/fixed size of a BundleId; this is stopgap for C compatibility
pub const MAX_BUNDLE_ID_SIZE: usize = 32;

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct BundleId {
    pub id: [u8; MAX_BUNDLE_ID_SIZE],
}
impl BundleId {
    pub fn empty(value: u8) -> Self {
        BundleId {
            id: [value; MAX_BUNDLE_ID_SIZE],
        }
    }
    pub fn new() -> Self {
        BundleId::empty(0)
    }
    pub fn is_zero(&self) -> bool {
        self.id == [0; MAX_BUNDLE_ID_SIZE]
    }
}

#[repr(C)]
pub struct BundleIdArray {
    pub ids: [BundleId; MAX_BUNDLES], // TODO(sleffler): placeholder
}
impl BundleIdArray {
    pub fn new() -> Self {
        BundleIdArray {
            ids: [BundleId::new(); MAX_BUNDLES],
        }
    }
    pub fn len(&self) -> usize {
        self.ids.iter().filter(|&id| !id.is_zero()).count()
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

// TODO(sleffler): Bundle should come from whomever implements install+uninstall
//   for ProcessManagerInterface
#[repr(C)]
pub struct Bundle {
    pub something: u32, // TODO(sleffler): placeholder
}
impl Bundle {
    pub fn new() -> Self {
        Bundle { something: 0 }
    }
}

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

// Abstract interface for injecting fakes, etc.
pub trait ProcessManagerInterface {
    fn install(&self, bundle: &Bundle) -> Result<(), ProcessManagerError>;
    fn uninstall(&self, bundle: &Bundle) -> Result<(), ProcessManagerError>;
    fn start(&self, bundle: &Bundle) -> Result<(), ProcessManagerError>;
    fn stop(&self, bundle: &Bundle) -> Result<(), ProcessManagerError>;
}

pub trait PackageManagementInterface<'a> {
    fn install(
        &mut self,
        bundle_id: &BundleId,
        bundle: &'a Bundle,
    ) -> Result<(), ProcessManagerError>;
    fn uninstall(&mut self, bundle_id: &BundleId) -> Result<(), ProcessManagerError>;
}

pub trait ProcessControlInterface {
    fn start(&mut self, bundle_id: &BundleId) -> Result<(), ProcessManagerError>;
    fn stop(&mut self, bundle_id: &BundleId) -> Result<(), ProcessManagerError>;
    fn get_running_bundles(&self) -> BundleIdArray;
}
