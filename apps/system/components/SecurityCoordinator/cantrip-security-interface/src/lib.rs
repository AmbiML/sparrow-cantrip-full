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

//! Cantrip OS Security Coordinator support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::{String, ToString};
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use core::str;
use log::trace;
use serde::{Deserialize, Serialize};

use sel4_sys::seL4_CPtr;

// NB: serde helper for arrays w/ >32 elements
//   c.f. https://github.com/serde-rs/serde/pull/1860
use serde_big_array::big_array;
big_array! { BigArray; }

// Size of the buffers used to pass serialized data between Rust <> C.
// The data structure size is bounded by the camkes ipc buffer (2K bytes!)
// and also by it being allocated on the stack of the rpc glue code.
// So we need to balance these against being able to handle all values.

const SECURITY_REQUEST_DATA_SIZE: usize = 2048;

pub const SECURITY_REPLY_DATA_SIZE: usize = 2048;
pub type SecurityReplyData = [u8; SECURITY_REPLY_DATA_SIZE];

// TODO(sleffler): temp constraint on value part of key-value pairs
pub const KEY_VALUE_DATA_SIZE: usize = 100;
pub type KeyValueData = [u8; KEY_VALUE_DATA_SIZE];

// NB: struct's marked repr(C) are processed by cbindgen to get a .h file
//   used in camkes C interfaces.

// Interface to any seL4 caapbility associated with the request.
pub trait SecurityCapability {
    fn get_container_cap(&self) -> Option<seL4_CPtr> { None }
    // TODO(sleffler): assert/log where no cap
    fn set_container_cap(&mut self, _cap: seL4_CPtr) {}
}

// SecurityRequestEcho
#[derive(Debug, Serialize, Deserialize)]
pub struct EchoRequest<'a> {
    pub value: &'a [u8],
}
impl<'a> SecurityCapability for EchoRequest<'a> {}

// SecurityRequestInstall
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallRequest {
    // NB: serde does not support a borrow
    pub pkg_contents: ObjDescBundle,
}
impl SecurityCapability for InstallRequest {
    fn get_container_cap(&self) -> Option<seL4_CPtr> { Some(self.pkg_contents.cnode) }
    fn set_container_cap(&mut self, cap: seL4_CPtr) { self.pkg_contents.cnode = cap; }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallResponse<'a> {
    pub bundle_id: &'a str,
}
impl<'a> SecurityCapability for InstallResponse<'a> {}

// SecurityRequestUninstall
#[derive(Debug, Serialize, Deserialize)]
pub struct UninstallRequest<'a> {
    pub bundle_id: &'a str,
}
impl<'a> SecurityCapability for UninstallRequest<'a> {}

// SecurityRequestSizeBuffer
#[derive(Debug, Serialize, Deserialize)]
pub struct SizeBufferRequest<'a> {
    pub bundle_id: &'a str,
}
impl<'a> SecurityCapability for SizeBufferRequest<'a> {}

#[derive(Debug, Serialize, Deserialize)]
pub struct SizeBufferResponse {
    pub buffer_size: usize,
}
impl SecurityCapability for SizeBufferResponse {}

// SecurityRequestGetManifest
#[derive(Debug, Serialize, Deserialize)]
pub struct GetManifestRequest<'a> {
    pub bundle_id: &'a str,
}
impl<'a> SecurityCapability for GetManifestRequest<'a> {}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetManifestResponse<'a> {
    pub manifest: &'a str,
}
impl<'a> SecurityCapability for GetManifestResponse<'a> {}

// SecurityRequestLoadApplication
#[derive(Debug, Serialize, Deserialize)]
pub struct LoadApplicationRequest<'a> {
    pub bundle_id: &'a str,
}
impl<'a> SecurityCapability for LoadApplicationRequest<'a> {}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadApplicationResponse {
    // Memory pages with verfied application contents.
    // TODO(sleffler) verify these are all Frames
    pub bundle_frames: ObjDescBundle,
}
impl SecurityCapability for LoadApplicationResponse {
    fn get_container_cap(&self) -> Option<seL4_CPtr> { Some(self.bundle_frames.cnode) }
    fn set_container_cap(&mut self, cap: seL4_CPtr) { self.bundle_frames.cnode = cap; }
}

// SecurityRequestLoadModel
#[derive(Debug, Serialize, Deserialize)]
pub struct LoadModelRequest<'a> {
    pub bundle_id: &'a str,
    pub model_id: &'a str,
}
impl<'a> SecurityCapability for LoadModelRequest<'a> {}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadModelResponse {
    // Memory pages with verified model contents.
    // TODO(sleffler) verify these are all Frames
    pub model_frames: ObjDescBundle,
}
impl SecurityCapability for LoadModelResponse {
    fn get_container_cap(&self) -> Option<seL4_CPtr> { Some(self.model_frames.cnode) }
    fn set_container_cap(&mut self, cap: seL4_CPtr) { self.model_frames.cnode = cap; }
}

// SecurityRequestReadKey
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadKeyRequest<'a> {
    pub bundle_id: &'a str,
    pub key: &'a str,
}
impl<'a> SecurityCapability for ReadKeyRequest<'a> {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadKeyResponse<'a> {
    pub value: &'a [u8],
}
impl<'a> SecurityCapability for ReadKeyResponse<'a> {}

// SecurityRequestWriteKey
#[derive(Debug, Serialize, Deserialize)]
pub struct WriteKeyRequest<'a> {
    pub bundle_id: &'a str,
    pub key: &'a str,
    pub value: &'a [u8],
}
impl<'a> SecurityCapability for WriteKeyRequest<'a> {}

// SecurityRequestDeleteKey
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteKeyRequest<'a> {
    pub bundle_id: &'a str,
    pub key: &'a str,
}
impl<'a> SecurityCapability for DeleteKeyRequest<'a> {}

// SecurityRequestTestMailbox
#[derive(Debug, Serialize, Deserialize)]
pub struct TestMailboxRequest {}
impl SecurityCapability for TestMailboxRequest {}

// SecurityRequestCapScan
#[derive(Debug, Serialize, Deserialize)]
pub struct CapScanRequest {}
impl SecurityCapability for CapScanRequest {}

// NB: this is the union of InstallInterface & StorageInterface because
//   the camkes-generated interface code uses basic C which does not
//   tolerate overlapping member names.
#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum SecurityRequestError {
    SreSuccess = 0,
    SreBundleIdInvalid,
    SreBundleDataInvalid,
    SreBundleNotFound,
    SreDeleteFirst,
    SreKeyNotFound,
    SrePackageBufferLenInvalid,
    SreValueInvalid,
    SreKeyInvalid,
    SreDeserializeFailed,
    SreSerializeFailed,
    SreCapAllocFailed,
    SreCapMoveFailed,
    SreObjCapInvalid,
    // Generic errors, mostly used in unit tests
    SreEchoFailed,
    SreInstallFailed,
    SreUninstallFailed,
    SreSizeBufferFailed,
    SreGetManifestFailed,
    SreLoadApplicationFailed,
    SreLoadModelFailed,
    SreReadFailed,
    SreWriteFailed,
    SreDeleteFailed,
    SreTestFailed,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecurityRequest {
    SrEcho = 0, // Security core replies with request payload

    SrInstall,   // Install package [pkg_buffer] -> bundle_id
    SrUninstall, // Uninstall package [bundle_id]

    SrSizeBuffer,      // Size application image [bundle_id] -> u32
    SrGetManifest,     // Return application manifest [bundle_id] -> String
    SrLoadApplication, // Load application [bundle_id]
    // TODO(sleffler): define <tag>?
    SrLoadModel, // Load ML model [bundle_id, <tag>]

    SrReadKey,   // Read key value [bundle_id, key] -> value
    SrWriteKey,  // Write key value [bundle_id, key, value]
    SrDeleteKey, // Delete key [bundle_id, key]

    SrTestMailbox, // Run mailbox tests
    SrCapScan,     // Dump contents CNode to console
}

// Interface to underlying facilities; also used to inject fakes for unit tests.
pub trait SecurityCoordinatorInterface {
    fn install(&mut self, pkg_contents: &ObjDescBundle) -> Result<String, SecurityRequestError>;
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), SecurityRequestError>;
    fn size_buffer(&self, bundle_id: &str) -> Result<usize, SecurityRequestError>;
    fn get_manifest(&self, bundle_id: &str) -> Result<String, SecurityRequestError>;
    fn load_application(&self, bundle_id: &str) -> Result<ObjDescBundle, SecurityRequestError>;
    fn load_model(
        &self,
        bundle_id: &str,
        model_id: &str,
    ) -> Result<ObjDescBundle, SecurityRequestError>;
    fn read_key(&self, bundle_id: &str, key: &str) -> Result<&KeyValueData, SecurityRequestError>;
    fn write_key(
        &mut self,
        bundle_id: &str,
        key: &str,
        value: &KeyValueData,
    ) -> Result<(), SecurityRequestError>;
    fn delete_key(&mut self, bundle_id: &str, key: &str) -> Result<(), SecurityRequestError>;
    fn test_mailbox(&mut self) -> Result<(), SecurityRequestError>;
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_request<T: Serialize + SecurityCapability>(
    request: SecurityRequest,
    request_args: &T,
    reply_buffer: &mut SecurityReplyData,
) -> Result<(), SecurityRequestError> {
    // NB: this assumes the SecurityCoordinator component is named "security".
    extern "C" {
        pub fn security_request(
            c_request: SecurityRequest,
            c_request_buffer_len: u32,
            c_request_buffer: *const u8,
            c_reply_buffer: *mut SecurityReplyData,
        ) -> SecurityRequestError;
    }
    trace!(
        "cantrip_security_request {:?} cap {:?}",
        &request,
        request_args.get_container_cap()
    );
    let mut request_buffer = [0u8; SECURITY_REQUEST_DATA_SIZE];
    let _ = postcard::to_slice(request_args, &mut request_buffer[..])
        .map_err(|_| SecurityRequestError::SreSerializeFailed)?;
    match unsafe {
        if let Some(cap) = request_args.get_container_cap() {
            let _cleanup = Camkes::set_request_cap(cap);
            security_request(
                request,
                request_buffer.len() as u32,
                request_buffer.as_ptr(),
                reply_buffer as *mut _,
            )
        } else {
            // NB: guard against a received badge being treated as an
            // outbound capability. This is needed because the code CAmkES
            // generates for security_request always enables possible xmit
            // of 1 capability.
            Camkes::clear_request_cap();
            security_request(
                request,
                request_buffer.len() as u32,
                request_buffer.as_ptr(),
                reply_buffer as *mut _,
            )
        }
    } {
        SecurityRequestError::SreSuccess => Ok(()),
        status => Err(status),
    }
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_echo(request: &str) -> Result<String, SecurityRequestError> {
    let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
    cantrip_security_request(
        SecurityRequest::SrEcho,
        &EchoRequest {
            value: request.as_bytes(),
        },
        reply,
    )
    .map(|_| String::from_utf8_lossy(&reply[..request.len()]).into_owned())
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_install(
    pkg_contents: &ObjDescBundle,
) -> Result<String, SecurityRequestError> {
    Camkes::debug_assert_slot_cnode(
        "cantrip_security_install",
        &Camkes::top_level_path(pkg_contents.cnode),
    );
    let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
    cantrip_security_request(
        SecurityRequest::SrInstall,
        &InstallRequest {
            pkg_contents: pkg_contents.clone(),
        },
        reply,
    )?;
    postcard::from_bytes::<String>(reply).map_err(|_| SecurityRequestError::SreDeserializeFailed)
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_uninstall(bundle_id: &str) -> Result<(), SecurityRequestError> {
    cantrip_security_request(
        SecurityRequest::SrUninstall,
        &UninstallRequest { bundle_id },
        &mut [0u8; SECURITY_REPLY_DATA_SIZE],
    )
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_size_buffer(bundle_id: &str) -> Result<usize, SecurityRequestError> {
    let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
    cantrip_security_request(
        SecurityRequest::SrSizeBuffer,
        &SizeBufferRequest { bundle_id },
        reply,
    )?;
    let response = postcard::from_bytes::<SizeBufferResponse>(reply)
        .map_err(|_| SecurityRequestError::SreDeserializeFailed)?;
    Ok(response.buffer_size)
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_get_manifest(bundle_id: &str) -> Result<String, SecurityRequestError> {
    let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
    cantrip_security_request(
        SecurityRequest::SrGetManifest,
        &GetManifestRequest { bundle_id },
        reply,
    )?;
    let response = postcard::from_bytes::<GetManifestResponse>(reply)
        .map_err(|_| SecurityRequestError::SreDeserializeFailed)?;
    Ok(response.manifest.to_string())
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_load_application(
    bundle_id: &str,
    container_slot: &CSpaceSlot,
) -> Result<ObjDescBundle, SecurityRequestError> {
    let _cleanup = container_slot.push_recv_path();
    // NB: SrLoadApplication returns a CNode with the application
    //   contents, make sure the receive slot is empty or it can
    //   silently fail.
    sel4_sys::debug_assert_slot_empty!(
        container_slot.slot,
        "Expected slot {:?} empty but has cap type {:?}",
        &container_slot.get_path(),
        sel4_sys::cap_identify(container_slot.slot)
    );

    let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
    cantrip_security_request(
        SecurityRequest::SrLoadApplication,
        &LoadApplicationRequest { bundle_id },
        reply,
    )?;
    if let Ok(mut response) = postcard::from_bytes::<LoadApplicationResponse>(reply) {
        sel4_sys::debug_assert_slot_cnode!(container_slot.slot);
        response.bundle_frames.cnode = container_slot.slot;
        Ok(response.bundle_frames)
    } else {
        Err(SecurityRequestError::SreDeserializeFailed)
    }
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_load_model(
    bundle_id: &str,
    model_id: &str,
    container_slot: &CSpaceSlot,
) -> Result<ObjDescBundle, SecurityRequestError> {
    let _cleanup = container_slot.push_recv_path();
    // NB: SrLoadApplication returns a CNode with the application
    //   contents, make sure the receive slot is empty or it can
    //   silently fail.
    sel4_sys::debug_assert_slot_empty!(
        container_slot.slot,
        "Expected slot {:?} empty but has cap type {:?}",
        &container_slot.get_path(),
        sel4_sys::cap_identify(container_slot.slot)
    );

    let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
    cantrip_security_request(
        SecurityRequest::SrLoadModel,
        &LoadModelRequest {
            bundle_id,
            model_id,
        },
        reply,
    )?;
    if let Ok(mut response) = postcard::from_bytes::<LoadModelResponse>(reply) {
        sel4_sys::debug_assert_slot_cnode!(container_slot.slot);
        response.model_frames.cnode = container_slot.slot;
        Ok(response.model_frames)
    } else {
        Err(SecurityRequestError::SreDeserializeFailed)
    }
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_read_key(
    bundle_id: &str,
    key: &str,
    keyval: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
    cantrip_security_request(
        SecurityRequest::SrReadKey,
        &ReadKeyRequest { bundle_id, key },
        reply,
    )?;
    let response = postcard::from_bytes::<ReadKeyResponse>(reply)
        .map_err(|_| SecurityRequestError::SreDeserializeFailed)?;
    keyval.copy_from_slice(response.value);
    Ok(())
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_write_key(
    bundle_id: &str,
    key: &str,
    value: &[u8],
) -> Result<(), SecurityRequestError> {
    cantrip_security_request(
        SecurityRequest::SrWriteKey,
        &WriteKeyRequest {
            bundle_id,
            key,
            value,
        },
        &mut [0u8; SECURITY_REPLY_DATA_SIZE],
    )
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_delete_key(bundle_id: &str, key: &str) -> Result<(), SecurityRequestError> {
    cantrip_security_request(
        SecurityRequest::SrDeleteKey,
        &DeleteKeyRequest { bundle_id, key },
        &mut [0u8; SECURITY_REPLY_DATA_SIZE],
    )
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_test_mailbox() -> Result<(), SecurityRequestError> {
    cantrip_security_request(
        SecurityRequest::SrTestMailbox,
        &TestMailboxRequest {},
        &mut [0u8; SECURITY_REPLY_DATA_SIZE],
    )
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_security_capscan() -> Result<(), SecurityRequestError> {
    cantrip_security_request(
        SecurityRequest::SrCapScan,
        &CapScanRequest {},
        &mut [0u8; SECURITY_REPLY_DATA_SIZE],
    )
}
