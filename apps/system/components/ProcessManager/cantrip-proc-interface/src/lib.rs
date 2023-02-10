// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Cantrip OS process management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::sel4_sys::seL4_CPtr;
use cantrip_security_interface::SecurityRequestError;
use core::str;
use log::trace;
use serde::{Deserialize, Serialize};

mod bundle_image;
pub use bundle_image::*;

const REQUEST_DATA_SIZE: usize = 128;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bundle {
    // NB: application & ML binaries use well-known paths relative to bundle_id
    // NB: ProcessManager owns loaded application's memory

    // Bundle id extracted from manifest
    pub app_id: String,

    // Size (bytes) of loaded application
    pub app_memory_size: u32,
}
impl Bundle {
    pub fn new(bundle_id: &str) -> Self {
        Bundle {
            app_id: String::from(bundle_id),
            app_memory_size: 0u32,
        }
    }
}

// Interface to underlying Bundle implementations. Mainly
// used to inject fakes for unit tests.
pub trait BundleImplInterface {
    fn start(&mut self) -> Result<(), ProcessManagerError>;
    fn stop(&mut self) -> Result<(), ProcessManagerError>;
    fn suspend(&self) -> Result<(), ProcessManagerError>;
    fn resume(&self) -> Result<(), ProcessManagerError>;
    fn capscan(&self) -> Result<(), ProcessManagerError>;
}

// NB: struct's marked repr(C) are processed by cbindgen to get a .h file
//   used in camkes C interfaces.

// Interface to any seL4 capability associated with the request.
pub trait Capability {
    fn get_container_cap(&self) -> Option<seL4_CPtr> { None }
    // TODO(sleffler): assert/log where no cap
    fn set_container_cap(&mut self, _cap: seL4_CPtr) {}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallRequest {
    // NB: serde does not support a borrow
    pub pkg_contents: ObjDescBundle,
}
impl Capability for InstallRequest {
    fn get_container_cap(&self) -> Option<seL4_CPtr> { Some(self.pkg_contents.cnode) }
    fn set_container_cap(&mut self, cap: seL4_CPtr) { self.pkg_contents.cnode = cap; }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallResponse<'a> {
    pub bundle_id: &'a str,
}
impl<'a> Capability for InstallResponse<'a> {}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallAppRequest<'a> {
    pub app_id: &'a str,
    // NB: serde does not support a borrow
    pub pkg_contents: ObjDescBundle,
}
impl<'a> Capability for InstallAppRequest<'a> {
    fn get_container_cap(&self) -> Option<seL4_CPtr> { Some(self.pkg_contents.cnode) }
    fn set_container_cap(&mut self, cap: seL4_CPtr) { self.pkg_contents.cnode = cap; }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UninstallRequest<'a> {
    pub bundle_id: &'a str,
}
impl<'a> Capability for UninstallRequest<'a> {}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartRequest<'a> {
    pub bundle_id: &'a str,
}
impl<'a> Capability for StartRequest<'a> {}

#[derive(Debug, Serialize, Deserialize)]
pub struct StopRequest<'a> {
    pub bundle_id: &'a str,
}
impl<'a> Capability for StopRequest<'a> {}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRunningBundlesRequest {}
impl Capability for GetRunningBundlesRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRunningBundlesResponse {
    pub bundle_ids: BundleIdArray,
}
impl Capability for GetRunningBundlesResponse {}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapScanRequest {}
impl Capability for CapScanRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapScanBundleRequest<'a> {
    pub bundle_id: &'a str,
}
impl<'a> Capability for CapScanBundleRequest<'a> {}

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProcessManagerError {
    Success = 0,
    BundleIdInvalid,
    PackageBufferLenInvalid,
    BundleNotFound,
    BundleFound,
    BundleRunning,
    BundleNotRunning,
    UnknownError,
    DeserializeError,
    SerializeError,
    ObjCapInvalid,
    // Generic errors, mostly for unit tests.
    InstallFailed,
    UninstallFailed,
    StartFailed,
    StopFailed,
    // TODO(sleffler): for use if/when ProcessManagerInterface grows
    SuspendFailed,
    ResumeFailed,
    CapScanFailed,
}

// Interface to underlying facilities (StorageManager, seL4); also
// used to inject fakes for unit tests.
pub trait ProcessManagerInterface {
    fn install(&mut self, pkg_contents: &ObjDescBundle) -> Result<String, ProcessManagerError>;
    fn install_app(
        &mut self,
        app_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), ProcessManagerError>;
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError>;
    fn start(
        &mut self,
        bundle: &Bundle,
    ) -> Result<Box<dyn BundleImplInterface>, ProcessManagerError>;
    fn stop(
        &mut self,
        bundle_impl: &mut dyn BundleImplInterface,
    ) -> Result<(), ProcessManagerError>;
    fn capscan(&self, bundle_impl: &dyn BundleImplInterface) -> Result<(), ProcessManagerError>;
}

// NB: bundle_id comes across the C interface as *const cstr_core::c_char
// and is converted to a &str using CStr::from_ptr().to_str().

pub trait PackageManagementInterface {
    fn install(&mut self, pkg_contents: &ObjDescBundle) -> Result<String, ProcessManagerError>;
    fn install_app(
        &mut self,
        app_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), ProcessManagerError>;
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError>;
}

pub trait ProcessControlInterface {
    fn start(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError>;
    fn stop(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError>;
    fn get_running_bundles(&self) -> Result<BundleIdArray, ProcessManagerError>;
    fn capscan(&self, bundle_id: &str) -> Result<(), ProcessManagerError>;
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageManagementRequest {
    PmrInstall = 0, // Install package [pkg_buffer] -> bundle_id
    PmrInstallApp,  // Install application [app_id, pkg_buffer]
    PmrUninstall,   // Uninstall package [bundle_id]
}

#[inline]
pub fn cantrip_pkg_mgmt_request<T: Serialize + Capability + core::fmt::Debug>(
    request: PackageManagementRequest,
    request_args: &T,
    reply_buffer: &mut RawBundleIdData,
) -> Result<(), ProcessManagerError> {
    extern "C" {
        pub fn pkg_mgmt_request(
            c_request: PackageManagementRequest,
            c_request_buffer_len: u32,
            c_request_buffer: *const u8,
            c_reply_buffer: *mut RawBundleIdData,
        ) -> ProcessManagerError;
    }
    trace!(
        "cantrip_pkg_mgmt_request {:?} cap {:?}",
        &request_args,
        request_args.get_container_cap()
    );
    let mut request_buffer = [0u8; REQUEST_DATA_SIZE];
    let request_slice = postcard::to_slice(request_args, &mut request_buffer[..])
        .map_err(ProcessManagerError::from)?;
    match unsafe {
        if let Some(cap) = request_args.get_container_cap() {
            let _cleanup = Camkes::set_request_cap(cap);
            pkg_mgmt_request(
                request,
                request_slice.len() as u32,
                request_slice.as_ptr(),
                reply_buffer as *mut _,
            )
        } else {
            // NB: guard against a received badge being treated as an
            // outbound capability. This is needed because the code CAmkES
            // generates for pkg_mgmt_request always enables possible xmit
            // of 1 capability.
            Camkes::clear_request_cap();
            pkg_mgmt_request(
                request,
                request_slice.len() as u32,
                request_slice.as_ptr(),
                reply_buffer as *mut _,
            )
        }
    } {
        ProcessManagerError::Success => Ok(()),
        status => Err(status),
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessControlRequest {
    PcrStart,             // [bundle_id]
    PcrStop,              // [bundle_id]
    PcrGetRunningBundles, // [] -> bundle_id

    PcrCapScan,       // []
    PcrCapScanBundle, // [bundle_id]
}

#[inline]
pub fn cantrip_proc_ctrl_request<T: Serialize + Capability + core::fmt::Debug>(
    request: ProcessControlRequest,
    request_args: &T,
    reply_buffer: &mut RawBundleIdData,
) -> Result<(), ProcessManagerError> {
    extern "C" {
        pub fn proc_ctrl_request(
            c_request: ProcessControlRequest,
            c_request_buffer_len: u32,
            c_request_buffer: *const u8,
            c_reply_buffer: *mut RawBundleIdData,
        ) -> ProcessManagerError;
    }
    trace!(
        "cantrip_proc_ctrl_request {:?} cap {:?}",
        &request_args,
        request_args.get_container_cap()
    );
    let mut request_buffer = [0u8; REQUEST_DATA_SIZE];
    let request_slice = postcard::to_slice(request_args, &mut request_buffer[..])
        .map_err(ProcessManagerError::from)?;
    match unsafe {
        if let Some(cap) = request_args.get_container_cap() {
            let _cleanup = Camkes::set_request_cap(cap);
            proc_ctrl_request(
                request,
                request_slice.len() as u32,
                request_slice.as_ptr(),
                reply_buffer as *mut _,
            )
        } else {
            // NB: guard against a received badge being treated as an
            // outbound capability. This is needed because the code CAmkES
            // generates for pkg_mgmt_request always enables possible xmit
            // of 1 capability.
            Camkes::clear_request_cap();
            proc_ctrl_request(
                request,
                request_slice.len() as u32,
                request_slice.as_ptr(),
                reply_buffer as *mut _,
            )
        }
    } {
        ProcessManagerError::Success => Ok(()),
        status => Err(status),
    }
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
pub fn cantrip_proc_ctrl_get_running_bundles() -> Result<BundleIdArray, ProcessManagerError> {
    let reply = &mut [0u8; RAW_BUNDLE_ID_DATA_SIZE];
    cantrip_proc_ctrl_request(
        ProcessControlRequest::PcrGetRunningBundles,
        &GetRunningBundlesRequest {},
        reply,
    )?;
    postcard::from_bytes::<BundleIdArray>(reply).map_err(ProcessManagerError::from)
}

#[inline]
pub fn cantrip_pkg_mgmt_install(
    pkg_contents: &ObjDescBundle,
) -> Result<String, ProcessManagerError> {
    Camkes::debug_assert_slot_cnode(
        "cantrip_pkg_mgmt_install",
        &Camkes::top_level_path(pkg_contents.cnode),
    );
    let reply = &mut [0u8; RAW_BUNDLE_ID_DATA_SIZE];
    cantrip_pkg_mgmt_request(
        PackageManagementRequest::PmrInstall,
        &InstallRequest {
            pkg_contents: pkg_contents.clone(),
        },
        reply,
    )?;
    postcard::from_bytes::<String>(reply).map_err(ProcessManagerError::from)
}

#[inline]
pub fn cantrip_pkg_mgmt_install_app(
    app_id: &str,
    pkg_contents: &ObjDescBundle,
) -> Result<(), ProcessManagerError> {
    Camkes::debug_assert_slot_cnode(
        "cantrip_pkg_mgmt_install_app",
        &Camkes::top_level_path(pkg_contents.cnode),
    );
    let reply = &mut [0u8; RAW_BUNDLE_ID_DATA_SIZE];
    cantrip_pkg_mgmt_request(
        PackageManagementRequest::PmrInstallApp,
        &InstallAppRequest {
            app_id,
            pkg_contents: pkg_contents.clone(),
        },
        reply,
    )
}

#[inline]
pub fn cantrip_pkg_mgmt_uninstall(bundle_id: &str) -> Result<(), ProcessManagerError> {
    cantrip_pkg_mgmt_request(
        PackageManagementRequest::PmrUninstall,
        &UninstallRequest { bundle_id },
        &mut [0u8; RAW_BUNDLE_ID_DATA_SIZE],
    )
}

#[inline]
pub fn cantrip_proc_ctrl_start(bundle_id: &str) -> Result<(), ProcessManagerError> {
    cantrip_proc_ctrl_request(
        ProcessControlRequest::PcrStart,
        &StartRequest { bundle_id },
        &mut [0u8; RAW_BUNDLE_ID_DATA_SIZE],
    )
}

#[inline]
pub fn cantrip_proc_ctrl_stop(bundle_id: &str) -> Result<(), ProcessManagerError> {
    cantrip_proc_ctrl_request(
        ProcessControlRequest::PcrStop,
        &StopRequest { bundle_id },
        &mut [0u8; RAW_BUNDLE_ID_DATA_SIZE],
    )
}

#[inline]
pub fn cantrip_proc_ctrl_capscan() -> Result<(), ProcessManagerError> {
    cantrip_proc_ctrl_request(
        ProcessControlRequest::PcrCapScan,
        &CapScanRequest {},
        &mut [0u8; RAW_BUNDLE_ID_DATA_SIZE],
    )
}

#[inline]
pub fn cantrip_proc_ctrl_capscan_bundle(bundle_id: &str) -> Result<(), ProcessManagerError> {
    cantrip_proc_ctrl_request(
        ProcessControlRequest::PcrCapScanBundle,
        &CapScanBundleRequest { bundle_id },
        &mut [0u8; RAW_BUNDLE_ID_DATA_SIZE],
    )
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
