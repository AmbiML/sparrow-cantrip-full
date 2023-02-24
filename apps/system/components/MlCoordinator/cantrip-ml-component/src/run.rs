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
#![allow(clippy::missing_safety_doc)]

extern crate alloc;

use alloc::string::ToString;
use cantrip_ml_coordinator::MLCoordinator;
use cantrip_ml_coordinator::ModelIdx;
use cantrip_ml_interface::CompleteJobsResponse;
use cantrip_ml_interface::MlCoordError;
use cantrip_ml_interface::MlCoordRequest;
use cantrip_ml_interface::MlCoordResponseData;
use cantrip_ml_shared::ImageId;
use cantrip_os_common::camkes::Camkes;
use cantrip_timer_interface::*;
use core::slice;
use log::error;
use spin::Mutex;

static mut CAMKES: Camkes = Camkes::new("MlCoordinator");
static ML_COORD: Mutex<MLCoordinator> = Mutex::new(MLCoordinator::new());

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static mut HEAP_MEMORY: [u8; 4 * 1024] = [0; 4 * 1024];
    CAMKES.pre_init(log::LevelFilter::Trace, &mut HEAP_MEMORY);
}

#[no_mangle]
pub unsafe extern "C" fn mlcoord__init() { ML_COORD.lock().init(); }

#[no_mangle]
pub unsafe extern "C" fn run() {
    loop {
        let mut completed = cantrip_timer_wait().unwrap();
        assert!(completed != 0);
        for i in 0..31 {
            let mask: TimerMask = 1 << i;
            if (completed & mask) != 0 {
                if let Err(e) = ML_COORD.lock().timer_completed(i as ModelIdx) {
                    error!("Error when trying to run periodic model: {:?}", e);
                }
                completed &= !mask;
                if completed == 0 {
                    break;
                }
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn host_req_handle() { ML_COORD.lock().handle_host_req_interrupt(); }

#[no_mangle]
pub unsafe extern "C" fn finish_handle() { ML_COORD.lock().handle_return_interrupt(); }

#[no_mangle]
pub unsafe extern "C" fn instruction_fault_handle() {
    ML_COORD.lock().handle_instruction_fault_interrupt();
}

#[no_mangle]
pub unsafe extern "C" fn data_fault_handle() { ML_COORD.lock().handle_data_fault_interrupt(); }

#[no_mangle]
pub unsafe extern "C" fn mlcoord_request(
    c_reques_buffer_len: u32,
    c_request_buffer: *const u8,
    c_reply: *mut MlCoordResponseData,
) -> MlCoordError {
    let _cleanup = Camkes::cleanup_request_cap();
    let request_buffer = slice::from_raw_parts(c_request_buffer, c_reques_buffer_len as usize);
    let request = match postcard::from_bytes::<MlCoordRequest>(request_buffer) {
        Ok(request) => request,
        Err(_) => return MlCoordError::MceDeserializeFailed,
    };

    match request {
        MlCoordRequest::CompletedJobs => completed_jobs_request(&mut *c_reply),
        MlCoordRequest::Oneshot {
            bundle_id,
            model_id,
        } => oneshot_request(bundle_id, model_id),
        MlCoordRequest::Periodic {
            bundle_id,
            model_id,
            rate_in_ms,
        } => periodic_request(bundle_id, model_id, rate_in_ms),
        MlCoordRequest::Cancel {
            bundle_id,
            model_id,
        } => cancel_request(bundle_id, model_id),
        MlCoordRequest::DebugState => {
            debug_state_request();
            Ok(())
        }
        MlCoordRequest::Capscan => {
            capscan_request();
            Ok(())
        }
    }
    .map_or_else(|e| e, |_v| MlCoordError::MceOk)
}

fn completed_jobs_request(reply_buffer: &mut MlCoordResponseData) -> Result<(), MlCoordError> {
    let job_mask = ML_COORD.lock().completed_jobs();
    let _ = postcard::to_slice(&CompleteJobsResponse { job_mask }, reply_buffer)
        .or(Err(MlCoordError::MceSerializeFailed))?;
    Ok(())
}

fn oneshot_request(bundle_id: &str, model_id: &str) -> Result<(), MlCoordError> {
    let image_id = ImageId {
        bundle_id: bundle_id.to_string(),
        model_id: model_id.to_string(),
    };
    ML_COORD.lock().oneshot(image_id)?;
    Ok(())
}

fn periodic_request(bundle_id: &str, model_id: &str, rate_in_ms: u32) -> Result<(), MlCoordError> {
    let image_id = ImageId {
        bundle_id: bundle_id.to_string(),
        model_id: model_id.to_string(),
    };
    ML_COORD.lock().periodic(image_id, rate_in_ms)?;
    Ok(())
}

fn cancel_request(bundle_id: &str, model_id: &str) -> Result<(), MlCoordError> {
    let image_id = ImageId {
        bundle_id: bundle_id.to_string(),
        model_id: model_id.to_string(),
    };
    ML_COORD.lock().cancel(&image_id)?;
    Ok(())
}

fn debug_state_request() { ML_COORD.lock().debug_state(); }

fn capscan_request() { let _ = Camkes::capscan(); }
