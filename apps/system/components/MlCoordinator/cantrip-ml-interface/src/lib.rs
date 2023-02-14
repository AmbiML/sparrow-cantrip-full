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
use log::trace;
use serde::{Deserialize, Serialize};

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_NBWait;
use sel4_sys::seL4_Wait;

pub type MlJobId = u32;
pub type MlJobMask = u32;

/// Errors that can occur when interacting with the MlCoordinator.
#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum MlCoordError {
    MceOk = 0,
    MceInvalidImage,
    MceInvalidTimer,
    MceLoadModelFailed,
    MceNoModelSlotsLeft,
    MceNoSuchModel,
    MceSerializeFailed,
    MceDeserializeFailed,
}

impl From<MlCoordError> for Result<(), MlCoordError> {
    fn from(err: MlCoordError) -> Result<(), MlCoordError> {
        if err == MlCoordError::MceOk {
            Ok(())
        } else {
            Err(err)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MlCoordRequest<'a> {
    // Returns a bit vector, where a 1 in bit N indicates job N has finished.
    // Outstanding completed jobs are reset to 0 during this call.
    CompletedJobs, // -> MlJobMask

    Oneshot {
        bundle_id: &'a str,
        model_id: &'a str,
    },
    Periodic {
        bundle_id: &'a str,
        model_id: &'a str,
        rate_in_ms: u32,
    },
    Cancel {
        bundle_id: &'a str,
        model_id: &'a str,
    },

    DebugState,
    Capscan,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteJobsResponse {
    pub job_mask: MlJobMask,
}

// Size of the data buffer used to pass a serialized MlCoordRequest between Rust <> C.
// The data structure size is bounded by the camkes ipc buffer (120 bytes!)
// and also by it being allocated on the stack of the rpc glue code.
const MLCOORD_REQUEST_DATA_SIZE: usize = 100;
// Size of the serialized response.
const MLCOORD_RESPONSE_DATA_SIZE: usize = core::mem::size_of::<CompleteJobsResponse>();
pub type MlCoordResponseData = [u8; MLCOORD_RESPONSE_DATA_SIZE];

#[inline]
#[allow(dead_code)]
pub fn cantrip_mlcoord_request(
    request: &MlCoordRequest,
    reply_buffer: &mut MlCoordResponseData,
) -> Result<(), MlCoordError> {
    extern "C" {
        pub fn mlcoord_request(
            c_request_buffer_len: u32,
            c_request_buffer: *const u8,
            c_reply_buffer: *mut MlCoordResponseData,
        ) -> MlCoordError;
    }
    trace!("cantrip_mlcoord_request {:?}", &request);
    let mut request_buffer = [0u8; MLCOORD_REQUEST_DATA_SIZE];
    let request_slice = postcard::to_slice(request, &mut request_buffer)
        .or(Err(MlCoordError::MceSerializeFailed))?;
    unsafe {
        mlcoord_request(request_slice.len() as u32, request_slice.as_ptr(), reply_buffer).into()
    }
}

#[inline]
pub fn cantrip_mlcoord_oneshot(bundle_id: &str, model_id: &str) -> Result<(), MlCoordError> {
    cantrip_mlcoord_request(
        &MlCoordRequest::Oneshot {
            bundle_id,
            model_id,
        },
        &mut [0u8; MLCOORD_RESPONSE_DATA_SIZE],
    )
}

#[inline]
pub fn cantrip_mlcoord_periodic(
    bundle_id: &str,
    model_id: &str,
    rate_in_ms: u32,
) -> Result<(), MlCoordError> {
    cantrip_mlcoord_request(
        &MlCoordRequest::Periodic {
            bundle_id,
            model_id,
            rate_in_ms,
        },
        &mut [0u8; MLCOORD_RESPONSE_DATA_SIZE],
    )
}

#[inline]
pub fn cantrip_mlcoord_cancel(bundle_id: &str, model_id: &str) -> Result<(), MlCoordError> {
    cantrip_mlcoord_request(
        &MlCoordRequest::Cancel {
            bundle_id,
            model_id,
        },
        &mut [0u8; MLCOORD_RESPONSE_DATA_SIZE],
    )
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
    let mut reply_buffer = [0u8; MLCOORD_RESPONSE_DATA_SIZE];
    cantrip_mlcoord_request(&MlCoordRequest::CompletedJobs, &mut reply_buffer)?;
    let reply = postcard::from_bytes::<CompleteJobsResponse>(&reply_buffer)
        .or(Err(MlCoordError::MceDeserializeFailed))?;
    Ok(reply.job_mask)
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
    let _ = cantrip_mlcoord_request(
        &MlCoordRequest::DebugState,
        &mut [0u8; MLCOORD_RESPONSE_DATA_SIZE],
    );
}

#[inline]
pub fn cantrip_mlcoord_capscan() -> Result<(), MlCoordError> {
    let _ =
        cantrip_mlcoord_request(&MlCoordRequest::Capscan, &mut [0u8; MLCOORD_RESPONSE_DATA_SIZE]);
    Ok(())
}
