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
use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::camkes;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use core::str;
use log::trace;
use num_enum::{FromPrimitive, IntoPrimitive};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use camkes::*;

use sel4_sys::seL4_CPtr;

// NB: serde helper for arrays w/ >32 elements
//   c.f. https://github.com/serde-rs/serde/pull/1860
use serde_big_array::big_array;
big_array! { BigArray; }

// Size of the buffers used to pass serialized data between Rust <> C.
// The data structure size is bounded by the camkes ipc buffer (2K bytes!)
// and also by it being allocated on the stack of the rpc glue code.
// So we need to balance these against being able to handle all values.

pub const SECURITY_REQUEST_DATA_SIZE: usize = 2048;

pub const SECURITY_REPLY_DATA_SIZE: usize = 2048;
pub type SecurityReplyData = [u8; SECURITY_REPLY_DATA_SIZE];

// TODO(sleffler): temp constraint on value part of key-value pairs
pub const KEY_VALUE_DATA_SIZE: usize = 100;
pub type KeyValueData = [u8; KEY_VALUE_DATA_SIZE];

pub type BundleIdArray = Vec<String>;

#[repr(usize)]
#[derive(Debug, Default, Eq, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum SecurityRequestError {
    Success = 0,
    BundleIdInvalid,
    BundleDataInvalid,
    BundleNotFound,
    DeleteFirst,
    KeyNotFound,
    PackageBufferLenInvalid,
    ValueInvalid,
    KeyInvalid,
    DeserializeFailed,
    SerializeFailed,
    CapAllocFailed,
    CapMoveFailed,
    ObjCapInvalid,
    #[default]
    UnknownError,
    // Generic errors, mostly used in unit tests
    EchoFailed,
    InstallFailed,
    UninstallFailed,
    SizeBufferFailed,
    GetManifestFailed,
    LoadApplicationFailed,
    LoadModelFailed,
    InstallModelFailed,
    GetPackagesFailed,
    ReadFailed,
    WriteFailed,
    DeleteFailed,
    TestFailed,
}
impl From<SecurityRequestError> for Result<(), SecurityRequestError> {
    fn from(err: SecurityRequestError) -> Result<(), SecurityRequestError> {
        if err == SecurityRequestError::Success {
            Ok(())
        } else {
            Err(err)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SecurityRequest<'a> {
    Echo(&'a str),                   // Security core sends back -> value
    Install(Cow<'a, ObjDescBundle>), // Install package -> bundle_id
    InstallApp {
        // Install application
        app_id: &'a str,
        pkg_contents: Cow<'a, ObjDescBundle>,
    },
    InstallModel {
        // Install model
        app_id: &'a str,
        model_id: &'a str,
        pkg_contents: Cow<'a, ObjDescBundle>,
    },
    Uninstall(&'a str), // Uninstall package
    GetPackages,        // Get package names -> BundleIdArray

    SizeBuffer(&'a str),      // Size application image -> u32
    GetManifest(&'a str),     // Application manifest -> String
    LoadApplication(&'a str), // Load application -> ObjDescBundle
    LoadModel {
        // Load ML model -> ObjDescBundle
        bundle_id: &'a str,
        model_id: &'a str,
    },

    ReadKey {
        // Read key value -> value
        bundle_id: &'a str,
        key: &'a str,
    },
    WriteKey {
        // Write key value
        bundle_id: &'a str,
        key: &'a str,
        value: &'a [u8],
    },
    DeleteKey {
        // Delete key
        bundle_id: &'a str,
        key: &'a str,
    },

    TestMailbox, // Exercise SecureCore mailbox
    CapScan,     // Dump CNode contents to console
}
impl<'a> SecurityRequest<'a> {
    fn get_container_cap(&self) -> Option<seL4_CPtr> {
        match self {
            SecurityRequest::Install(pkg_contents)
            | SecurityRequest::InstallApp {
                app_id: _,
                pkg_contents,
            }
            | SecurityRequest::InstallModel {
                app_id: _,
                model_id: _,
                pkg_contents,
            } => Some(pkg_contents.cnode),

            SecurityRequest::Echo(_)
            | SecurityRequest::Uninstall(_)
            | SecurityRequest::GetPackages
            | SecurityRequest::SizeBuffer(_)
            | SecurityRequest::GetManifest(_)
            | SecurityRequest::LoadApplication(_)
            | SecurityRequest::LoadModel {
                bundle_id: _,
                model_id: _,
            }
            | SecurityRequest::ReadKey {
                bundle_id: _,
                key: _,
            }
            | SecurityRequest::WriteKey {
                bundle_id: _,
                key: _,
                value: _,
            }
            | SecurityRequest::DeleteKey {
                bundle_id: _,
                key: _,
            }
            | SecurityRequest::TestMailbox
            | SecurityRequest::CapScan => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EchoResponse {
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallResponse {
    pub bundle_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPackagesResponse {
    pub bundle_ids: BundleIdArray,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SizeBufferResponse {
    pub buffer_size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetManifestResponse {
    pub manifest: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadApplicationResponse {
    // Memory pages with verfied application contents.
    // TODO(sleffler) verify these are all Frames
    pub bundle_frames: ObjDescBundle,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadModelResponse {
    // Memory pages with verified model contents.
    // TODO(sleffler) verify these are all Frames
    pub model_frames: ObjDescBundle,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadKeyResponse {
    #[serde(with = "BigArray")]
    pub value: KeyValueData,
}

// Interface to underlying facilities; also used to inject fakes for unit tests.
pub trait SecurityCoordinatorInterface {
    fn install(&mut self, pkg_contents: &ObjDescBundle) -> Result<String, SecurityRequestError>;
    fn install_app(
        &mut self,
        app_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), SecurityRequestError>;
    fn install_model(
        &mut self,
        app_id: &str,
        model_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), SecurityRequestError>;
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), SecurityRequestError>;
    fn get_packages(&self) -> Result<BundleIdArray, SecurityRequestError>;
    fn size_buffer(&self, bundle_id: &str) -> Result<usize, SecurityRequestError>;
    fn get_manifest(&self, bundle_id: &str) -> Result<String, SecurityRequestError>;
    fn load_application(&mut self, bundle_id: &str) -> Result<ObjDescBundle, SecurityRequestError>;
    fn load_model(
        &mut self,
        bundle_id: &str,
        model_id: &str,
    ) -> Result<ObjDescBundle, SecurityRequestError>;
    fn read_key(&self, bundle_id: &str, key: &str) -> Result<&KeyValueData, SecurityRequestError>;
    fn write_key(
        &mut self,
        bundle_id: &str,
        key: &str,
        value: &[u8],
    ) -> Result<(), SecurityRequestError>;
    fn delete_key(&mut self, bundle_id: &str, key: &str) -> Result<(), SecurityRequestError>;
    fn test_mailbox(&mut self) -> Result<(), SecurityRequestError>;
}

#[inline]
pub fn cantrip_security_request<T: DeserializeOwned>(
    request: &SecurityRequest,
) -> Result<T, SecurityRequestError> {
    trace!(
        "cantrip_security_request {:?} cap {:?}",
        &request,
        request.get_container_cap()
    );
    let (request_slice, reply_slice) =
        rpc_shared_buffer_mut!(security).split_at_mut(SECURITY_REQUEST_DATA_SIZE);
    let _ = postcard::to_slice(request, request_slice)
        .or(Err(SecurityRequestError::SerializeFailed))?;
    match rpc_shared_send!(security, request.get_container_cap()) {
        0 => postcard::from_bytes(reply_slice).or(Err(SecurityRequestError::DeserializeFailed)),
        err => Err(err.into()),
    }
}

#[inline]
pub fn cantrip_security_echo(request: &str) -> Result<String, SecurityRequestError> {
    cantrip_security_request(&SecurityRequest::Echo(request)).map(|reply: EchoResponse| reply.value)
}

#[inline]
pub fn cantrip_security_install(
    pkg_contents: &ObjDescBundle,
) -> Result<String, SecurityRequestError> {
    Camkes::debug_assert_slot_cnode(
        "cantrip_security_install",
        &Camkes::top_level_path(pkg_contents.cnode),
    );
    cantrip_security_request(&SecurityRequest::Install(Cow::Borrowed(pkg_contents)))
        .map(|reply: InstallResponse| reply.bundle_id)
}

#[inline]
pub fn cantrip_security_install_application(
    app_id: &str,
    pkg_contents: &ObjDescBundle,
) -> Result<(), SecurityRequestError> {
    Camkes::debug_assert_slot_cnode(
        "cantrip_security_install_application",
        &Camkes::top_level_path(pkg_contents.cnode),
    );
    cantrip_security_request(&SecurityRequest::InstallApp {
        app_id,
        pkg_contents: Cow::Borrowed(pkg_contents),
    })
}

#[inline]
pub fn cantrip_security_install_model(
    app_id: &str,
    model_id: &str,
    pkg_contents: &ObjDescBundle,
) -> Result<(), SecurityRequestError> {
    Camkes::debug_assert_slot_cnode(
        "cantrip_security_install_model",
        &Camkes::top_level_path(pkg_contents.cnode),
    );
    cantrip_security_request(&SecurityRequest::InstallModel {
        app_id,
        model_id,
        pkg_contents: Cow::Borrowed(pkg_contents),
    })
}

#[inline]
pub fn cantrip_security_uninstall(bundle_id: &str) -> Result<(), SecurityRequestError> {
    cantrip_security_request(&SecurityRequest::Uninstall(bundle_id))
}

#[inline]
pub fn cantrip_security_get_packages() -> Result<BundleIdArray, SecurityRequestError> {
    cantrip_security_request(&SecurityRequest::GetPackages)
        .map(|reply: GetPackagesResponse| reply.bundle_ids)
}

#[inline]
pub fn cantrip_security_size_buffer(bundle_id: &str) -> Result<usize, SecurityRequestError> {
    cantrip_security_request(&SecurityRequest::SizeBuffer(bundle_id))
        .map(|reply: SizeBufferResponse| reply.buffer_size)
}

#[inline]
pub fn cantrip_security_get_manifest(bundle_id: &str) -> Result<String, SecurityRequestError> {
    cantrip_security_request(&SecurityRequest::GetManifest(bundle_id))
        .map(|reply: GetManifestResponse| reply.manifest)
}

#[inline]
pub fn cantrip_security_load_application(
    bundle_id: &str,
    container_slot: &CSpaceSlot,
) -> Result<ObjDescBundle, SecurityRequestError> {
    let _cleanup = container_slot.push_recv_path();
    // NB: LoadApplication returns a CNode with the application
    //   contents, make sure the receive slot is empty or it can
    //   silently fail.
    sel4_sys::debug_assert_slot_empty!(
        container_slot.slot,
        "Expected slot {:?} empty but has {:?}",
        &container_slot.get_path(),
        sel4_sys::cap_identify(container_slot.slot)
    );

    let mut reply = cantrip_security_request::<LoadApplicationResponse>(
        &SecurityRequest::LoadApplication(bundle_id),
    )?;
    sel4_sys::debug_assert_slot_cnode!(container_slot.slot);
    reply.bundle_frames.cnode = container_slot.slot;
    Ok(reply.bundle_frames)
}

#[inline]
pub fn cantrip_security_load_model(
    bundle_id: &str,
    model_id: &str,
    container_slot: &CSpaceSlot,
) -> Result<ObjDescBundle, SecurityRequestError> {
    let _cleanup = container_slot.push_recv_path();
    // NB: SrLoadModel returns a CNode with the model contents, make
    // sure the receive slot is empty or it can silently fail.
    sel4_sys::debug_assert_slot_empty!(
        container_slot.slot,
        "Expected slot {:?} empty but has {:?}",
        &container_slot.get_path(),
        sel4_sys::cap_identify(container_slot.slot)
    );

    let mut reply = cantrip_security_request::<LoadModelResponse>(&SecurityRequest::LoadModel {
        bundle_id,
        model_id,
    })?;
    sel4_sys::debug_assert_slot_cnode!(container_slot.slot);
    reply.model_frames.cnode = container_slot.slot;
    Ok(reply.model_frames)
}

#[inline]
pub fn cantrip_security_read_key(
    bundle_id: &str,
    key: &str,
) -> Result<KeyValueData, SecurityRequestError> {
    cantrip_security_request(&SecurityRequest::ReadKey { bundle_id, key })
        .map(|reply: ReadKeyResponse| reply.value)
}

#[inline]
pub fn cantrip_security_write_key(
    bundle_id: &str,
    key: &str,
    value: &[u8],
) -> Result<(), SecurityRequestError> {
    cantrip_security_request(&SecurityRequest::WriteKey {
        bundle_id,
        key,
        value,
    })
}

#[inline]
pub fn cantrip_security_delete_key(bundle_id: &str, key: &str) -> Result<(), SecurityRequestError> {
    cantrip_security_request(&SecurityRequest::DeleteKey { bundle_id, key })
}

#[inline]
pub fn cantrip_security_test_mailbox() -> Result<(), SecurityRequestError> {
    cantrip_security_request(&SecurityRequest::TestMailbox)
}

#[inline]
pub fn cantrip_security_capscan() -> Result<(), SecurityRequestError> {
    cantrip_security_request(&SecurityRequest::CapScan)
}
