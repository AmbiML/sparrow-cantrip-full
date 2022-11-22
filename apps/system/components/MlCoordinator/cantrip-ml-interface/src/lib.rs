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

#![no_std]
use cantrip_os_common::sel4_sys;
use cstr_core::CString;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_NBWait;
use sel4_sys::seL4_Wait;

pub type MlJobId = u32;
pub type MlJobMask = u32;

/// Errors that can occur when interacting with the MlCoordinator.
#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum MlCoordError {
    MlCoordOk,
    InvalidModelId,
    InvalidBundleId,
    InvalidImage,
    InvalidTimer,
    LoadModelFailed,
    NoModelSlotsLeft,
    NoSuchModel,
}

impl From<MlCoordError> for Result<(), MlCoordError> {
    fn from(err: MlCoordError) -> Result<(), MlCoordError> {
        if err == MlCoordError::MlCoordOk {
            Ok(())
        } else {
            Err(err)
        }
    }
}

#[inline]
pub fn cantrip_mlcoord_oneshot(bundle_id: &str, model_id: &str) -> Result<(), MlCoordError> {
    extern "C" {
        // NB: this assumes the MlCoordinator component is named "mlcoord".
        fn mlcoord_oneshot(
            c_bundle_id: *const cstr_core::c_char,
            c_model_id: *const cstr_core::c_char,
        ) -> MlCoordError;
    }
    let bundle_id_cstr = CString::new(bundle_id).map_err(|_| MlCoordError::InvalidBundleId)?;
    let model_id_cstr = CString::new(model_id).map_err(|_| MlCoordError::InvalidModelId)?;

    unsafe { mlcoord_oneshot(bundle_id_cstr.as_ptr(), model_id_cstr.as_ptr()) }.into()
}

#[inline]
pub fn cantrip_mlcoord_periodic(
    bundle_id: &str,
    model_id: &str,
    rate_in_ms: u32,
) -> Result<(), MlCoordError> {
    extern "C" {
        fn mlcoord_periodic(
            c_bundle_id: *const cstr_core::c_char,
            c_model_id: *const cstr_core::c_char,
            rate_in_ms: u32,
        ) -> MlCoordError;
    }
    let bundle_id_cstr = CString::new(bundle_id).map_err(|_| MlCoordError::InvalidBundleId)?;
    let model_id_cstr = CString::new(model_id).map_err(|_| MlCoordError::InvalidModelId)?;

    unsafe { mlcoord_periodic(bundle_id_cstr.as_ptr(), model_id_cstr.as_ptr(), rate_in_ms) }.into()
}

#[inline]
pub fn cantrip_mlcoord_cancel(bundle_id: &str, model_id: &str) -> Result<(), MlCoordError> {
    extern "C" {
        fn mlcoord_cancel(
            c_bundle_id: *const cstr_core::c_char,
            c_model_id: *const cstr_core::c_char,
        ) -> MlCoordError;
    }
    let bundle_id_cstr = CString::new(bundle_id).map_err(|_| MlCoordError::InvalidBundleId)?;
    let model_id_cstr = CString::new(model_id).map_err(|_| MlCoordError::InvalidModelId)?;

    unsafe { mlcoord_cancel(bundle_id_cstr.as_ptr(), model_id_cstr.as_ptr()) }.into()
}

/// Returns the cptr for the notification object used to signal events.
#[inline]
pub fn cantrip_mlcoord_notification() -> seL4_CPtr {
    extern "C" {
        fn mlcoord_notification() -> seL4_CPtr;
    }
    unsafe { mlcoord_notification() }
}

/// Returns a bitmask of job id's registered with cantrip_mlcoord_oneshot
/// and cantrip_mlcoord_periodic that have expired.
#[inline]
pub fn cantrip_mlcoord_completed_jobs() -> Result<MlJobMask, MlCoordError> {
    extern "C" {
        fn mlcoord_completed_jobs() -> u32;
    }
    Ok(unsafe { mlcoord_completed_jobs() } as MlJobMask)
}

/// Waits for the next pending job for the client. If a job completes
/// the associated job id is returned.
#[inline]
pub fn cantrip_mlcoord_wait() -> Result<MlJobMask, MlCoordError> {
    unsafe {
        seL4_Wait(cantrip_mlcoord_notification(), core::ptr::null_mut());
    }
    cantrip_mlcoord_completed_jobs()
}

/// Returns a bitmask of completed jobs. Note this is non-blocking; to
/// wait for one or more jobs to complete use cantrip_mlcoord_wait.
#[inline]
pub fn cantrip_mlcoord_poll() -> Result<MlJobMask, MlCoordError> {
    unsafe {
        seL4_NBWait(cantrip_mlcoord_notification(), core::ptr::null_mut());
    }
    cantrip_mlcoord_completed_jobs()
}

#[inline]
pub fn cantrip_mlcoord_debug_state() {
    extern "C" {
        fn mlcoord_debug_state();
    }
    unsafe { mlcoord_debug_state() };
}

#[inline]
pub fn cantrip_mlcoord_capscan() -> Result<(), MlCoordError> {
    extern "C" {
        fn mlcoord_capscan();
    }
    unsafe { mlcoord_capscan() };
    Ok(())
}
