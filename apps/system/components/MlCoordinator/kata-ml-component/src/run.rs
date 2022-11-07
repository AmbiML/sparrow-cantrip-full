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

use alloc::string::String;
use cstr_core::CStr;
use cantrip_ml_coordinator::MLCoordinator;
use cantrip_ml_coordinator::ModelIdx;
use cantrip_ml_interface::MlCoordError;
use cantrip_ml_shared::ImageId;
use cantrip_os_common::camkes::Camkes;
use cantrip_timer_interface::*;
use log::error;
use spin::Mutex;

static mut CAMKES: Camkes = Camkes::new("MlCoordinator");
static mut ML_COORD: Mutex<MLCoordinator> = Mutex::new(MLCoordinator::new());

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
        cantrip_timer_wait();
        let completed = cantrip_timer_completed_timers();

        for i in 0..31 {
            let idx: u32 = 1 << i;
            if completed & idx != 0 {
                if let Err(e) = ML_COORD.lock().timer_completed(i as ModelIdx) {
                    error!("Error when trying to run periodic model: {:?}", e);
                }
            }
        }
    }
}

unsafe fn validate_ids(
    c_bundle_id: *const cstr_core::c_char,
    c_model_id: *const cstr_core::c_char,
) -> Result<ImageId, MlCoordError> {
    let bundle_id = CStr::from_ptr(c_bundle_id)
        .to_str()
        .map_err(|_| MlCoordError::InvalidBundleId)?;
    let model_id = CStr::from_ptr(c_model_id)
        .to_str()
        .map_err(|_| MlCoordError::InvalidModelId)?;
    Ok(ImageId {
        bundle_id: String::from(bundle_id),
        model_id: String::from(model_id),
    })
}

#[no_mangle]
pub unsafe extern "C" fn mlcoord_oneshot(
    c_bundle_id: *const cstr_core::c_char,
    c_model_id: *const cstr_core::c_char,
) -> MlCoordError {
    let id = match validate_ids(c_bundle_id, c_model_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    if let Err(e) = ML_COORD.lock().oneshot(id) {
        return e;
    }

    MlCoordError::MlCoordOk
}

#[no_mangle]
pub unsafe extern "C" fn mlcoord_periodic(
    c_bundle_id: *const cstr_core::c_char,
    c_model_id: *const cstr_core::c_char,
    rate_in_ms: u32,
) -> MlCoordError {
    let id = match validate_ids(c_bundle_id, c_model_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    if let Err(e) = ML_COORD.lock().periodic(id, rate_in_ms) {
        return e;
    }

    MlCoordError::MlCoordOk
}

#[no_mangle]
pub unsafe extern "C" fn mlcoord_cancel(
    c_bundle_id: *const cstr_core::c_char,
    c_model_id: *const cstr_core::c_char,
) -> MlCoordError {
    let id = match validate_ids(c_bundle_id, c_model_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    if let Err(e) = ML_COORD.lock().cancel(&id) {
        return e;
    }

    MlCoordError::MlCoordOk
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
pub unsafe extern "C" fn mlcoord_debug_state() { ML_COORD.lock().debug_state(); }

#[no_mangle]
pub unsafe extern "C" fn mlcoord_capscan() { let _ = Camkes::capscan(); }
