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

use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use cstr_core::CString;

use sel4_sys::seL4_CPtr;

#[repr(C)]
#[derive(Eq, PartialEq)]
pub enum SDKManagerError {
    SmSuccess = 0,
    SmSerializeFailed,
    SmAppIdInvalid,
    SmGetEndpointFailed,
    SmReleaseEndpointFailed,
}

impl From<SDKManagerError> for Result<(), SDKManagerError> {
    fn from(err: SDKManagerError) -> Result<(), SDKManagerError> {
        if err == SDKManagerError::SmSuccess {
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

    extern "C" {
        pub fn sdk_manager_get_endpoint(c_bundle_id: *const cstr_core::c_char) -> SDKManagerError;
    }
    let cstr = CString::new(app_id).map_err(|_| SDKManagerError::SmSerializeFailed)?;
    unsafe { sdk_manager_get_endpoint(cstr.as_ptr()) }.into()
}

#[inline]
pub fn cantrip_sdk_manager_release_endpoint(app_id: &str) -> Result<(), SDKManagerError> {
    extern "C" {
        pub fn sdk_manager_release_endpoint(
            c_bundle_id: *const cstr_core::c_char,
        ) -> SDKManagerError;
    }
    let cstr = CString::new(app_id).map_err(|_| SDKManagerError::SmSerializeFailed)?;
    unsafe { sdk_manager_release_endpoint(cstr.as_ptr()) }.into()
}

#[inline]
pub fn cantrip_sdk_manager_capscan() -> Result<(), SDKManagerError> {
    extern "C" {
        pub fn sdk_manager_capscan();
    }
    unsafe { sdk_manager_capscan() }
    Ok(())
}
