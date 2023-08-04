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
use cantrip_os_common::camkes;
use cantrip_os_common::sel4_sys;
use log::trace;
use num_enum::{FromPrimitive, IntoPrimitive};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use camkes::*;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_NBWait;
use sel4_sys::seL4_Wait;

pub type MlJobId = u32;
pub type MlJobMask = u32;

use serde_big_array::big_array;
big_array! { BigArray; }

// TODO(sleffler): too small for planned demo's
pub const MAX_OUTPUT_DATA: usize = 64;

#[derive(Debug, Serialize, Deserialize)]
pub struct MlOutput {
    pub jobnum: usize, // unique value per model run
    pub return_code: u32,
    pub epc: Option<u32>, // NB: Springbok only
    #[serde(with = "BigArray")]
    pub data: [u8; MAX_OUTPUT_DATA],
}

/// Errors that can occur when interacting with the MlCoordinator.
#[repr(usize)]
#[derive(Debug, Default, Eq, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum MlCoordError {
    Success = 0,
    InvalidImage,
    InvalidTimer,
    LoadModelFailed,
    NoModelSlotsLeft,
    NoSuchModel,
    NoOutputHeader,
    SerializeError,
    DeserializeError,
    #[default]
    UnknownError,
}
impl From<MlCoordError> for Result<(), MlCoordError> {
    fn from(err: MlCoordError) -> Result<(), MlCoordError> {
        if err == MlCoordError::Success {
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

    // Returns the relevant OutputHeader & and any indirect data.
    GetOutput {
        // -> MlOutput
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

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOutputResponse {
    pub output: MlOutput,
}

pub const MLCOORD_REQUEST_DATA_SIZE: usize = 100;

#[inline]
fn cantrip_mlcoord_request<T: DeserializeOwned>(
    request: &MlCoordRequest,
) -> Result<T, MlCoordError> {
    trace!("cantrip_mlcoord_request {:?}", &request);
    let (request_buffer, reply_slice) = rpc_basic_buffer!().split_at_mut(MLCOORD_REQUEST_DATA_SIZE);
    let request_slice =
        postcard::to_slice(request, request_buffer).or(Err(MlCoordError::SerializeError))?;
    match rpc_basic_send!(mlcoord, request_slice.len()).0.into() {
        MlCoordError::Success => {
            let reply =
                postcard::from_bytes(reply_slice).or(Err(MlCoordError::DeserializeError))?;
            Ok(reply)
        }
        err => Err(err),
    }
}

#[inline]
pub fn cantrip_mlcoord_oneshot(bundle_id: &str, model_id: &str) -> Result<(), MlCoordError> {
    cantrip_mlcoord_request(&MlCoordRequest::Oneshot {
        bundle_id,
        model_id,
    })
}

#[inline]
pub fn cantrip_mlcoord_periodic(
    bundle_id: &str,
    model_id: &str,
    rate_in_ms: u32,
) -> Result<(), MlCoordError> {
    cantrip_mlcoord_request(&MlCoordRequest::Periodic {
        bundle_id,
        model_id,
        rate_in_ms,
    })
}

#[inline]
pub fn cantrip_mlcoord_cancel(bundle_id: &str, model_id: &str) -> Result<(), MlCoordError> {
    cantrip_mlcoord_request(&MlCoordRequest::Cancel {
        bundle_id,
        model_id,
    })
}

/// Returns a bitmask of job id's registered with cantrip_mlcoord_oneshot
/// and cantrip_mlcoord_periodic that have expired.
#[inline]
pub fn cantrip_mlcoord_completed_jobs() -> Result<MlJobMask, MlCoordError> {
    cantrip_mlcoord_request(&MlCoordRequest::CompletedJobs)
        .map(|reply: CompleteJobsResponse| reply.job_mask)
}

/// Returns the OutputHeader & indirect data for the specified job.
#[inline]
pub fn cantrip_mlcoord_get_output(
    bundle_id: &str,
    model_id: &str,
) -> Result<MlOutput, MlCoordError> {
    cantrip_mlcoord_request(&MlCoordRequest::GetOutput {
        bundle_id,
        model_id,
    })
    .map(|reply: GetOutputResponse| reply.output)
}

/// Waits for the next pending job for the client. If a job completes
/// the associated job id is returned.
#[inline]
pub fn cantrip_mlcoord_wait() -> Result<MlJobMask, MlCoordError> {
    unsafe {
        extern "Rust" {
            static MLCOORD_INTERFACE_NOTIFICATION: seL4_CPtr;
        }
        seL4_Wait(MLCOORD_INTERFACE_NOTIFICATION, core::ptr::null_mut());
    }
    cantrip_mlcoord_completed_jobs()
}

/// Returns a bitmask of completed jobs. Note this is non-blocking; to
/// wait for one or more jobs to complete use cantrip_mlcoord_wait.
#[inline]
pub fn cantrip_mlcoord_poll() -> Result<MlJobMask, MlCoordError> {
    unsafe {
        extern "Rust" {
            static MLCOORD_INTERFACE_NOTIFICATION: seL4_CPtr;
        }
        seL4_NBWait(MLCOORD_INTERFACE_NOTIFICATION, core::ptr::null_mut());
    }
    cantrip_mlcoord_completed_jobs()
}

#[inline]
pub fn cantrip_mlcoord_debug_state() {
    let _ = cantrip_mlcoord_request::<()>(&MlCoordRequest::DebugState);
}

#[inline]
pub fn cantrip_mlcoord_capscan() -> Result<(), MlCoordError> {
    cantrip_mlcoord_request(&MlCoordRequest::Capscan)
}
