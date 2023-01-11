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

//! CantripOS SDK manager interfaces

#![cfg_attr(not(test), no_std)]

use cantrip_os_common::camkes;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use num_enum::{FromPrimitive, IntoPrimitive};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use camkes::*;

use sel4_sys::seL4_CPtr;

#[repr(usize)]
#[derive(Debug, Default, Eq, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum SDKManagerError {
    Success = 0,
    DeserializeFailed,
    SerializeFailed,
    AppIdInvalid,
    GetEndpointFailed,
    ReleaseEndpointFailed,
    #[default]
    UnknownError,
}
impl From<SDKManagerError> for Result<(), SDKManagerError> {
    fn from(err: SDKManagerError) -> Result<(), SDKManagerError> {
        if err == SDKManagerError::Success {
            Ok(())
        } else {
            Err(err)
        }
    }
}

/// Rust manager interface for the SDKRuntime.
pub trait SDKManagerInterface {
    /// Returns a badged endpoint capability setup for making
    /// SDKRuntime requests. The endpoint is meant to be returned
    /// to the caller attached to the IPC buffer. SDKRuntime requests
    /// are rejected unless they arrive through a properly-badged endpoint.
    fn get_endpoint(&mut self, app_id: &str) -> Result<seL4_CPtr, SDKManagerError>;

    /// Remove an application badge setup with get_endpoint.
    fn release_endpoint(&mut self, app_id: &str) -> Result<(), SDKManagerError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SDKManagerRequest<'a> {
    GetEndpoint(&'a str), // -> cap_endpoint
    ReleaseEndpoint(&'a str),
    Capscan,
}

pub const SDK_MANAGER_REQUEST_DATA_SIZE: usize = 128;

#[inline]
fn cantrip_sdk_manager_request<D: DeserializeOwned>(
    request: &SDKManagerRequest,
) -> Result<D, SDKManagerError> {
    let (request_buffer, reply_slice) =
        rpc_basic_buffer!().split_at_mut(SDK_MANAGER_REQUEST_DATA_SIZE);
    let request_slice =
        postcard::to_slice(request, request_buffer).or(Err(SDKManagerError::SerializeFailed))?;
    // XXX returned cap
    match rpc_basic_send!(sdk_manager, request_slice.len()).0.into() {
        SDKManagerError::Success => {
            let reply =
                postcard::from_bytes(reply_slice).or(Err(SDKManagerError::DeserializeFailed))?;
            Ok(reply)
        }
        err => Err(err),
    }
}

#[inline]
pub fn cantrip_sdk_manager_get_endpoint(
    app_id: &str,
    container_slot: &CSpaceSlot,
) -> Result<(), SDKManagerError> {
    let _cleanup = container_slot.push_recv_path();
    // NB: make sure the receive slot is empty or the cap will be dropped.
    sel4_sys::debug_assert_slot_empty!(
        container_slot.slot,
        "Expected slot {:?} empty but has cap type {:?}",
        &container_slot.get_path(),
        sel4_sys::cap_identify(container_slot.slot)
    );

    cantrip_sdk_manager_request(&SDKManagerRequest::GetEndpoint(app_id))
}

#[inline]
pub fn cantrip_sdk_manager_release_endpoint(app_id: &str) -> Result<(), SDKManagerError> {
    cantrip_sdk_manager_request(&SDKManagerRequest::ReleaseEndpoint(app_id))
}

#[inline]
pub fn cantrip_sdk_manager_capscan() -> Result<(), SDKManagerError> {
    cantrip_sdk_manager_request(&SDKManagerRequest::Capscan)
}
