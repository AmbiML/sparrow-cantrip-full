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
//error[E0658]: dereferencing raw mutable pointers in statics is unstable
#![feature(const_mut_refs)]

extern crate alloc;
use alloc::string::ToString;
use cantrip_ml_coordinator::MLCoordinator;
use cantrip_ml_coordinator::ModelIdx;
use cantrip_ml_interface::CompleteJobsResponse;
use cantrip_ml_interface::GetOutputResponse;
use cantrip_ml_interface::MlCoordError;
use cantrip_ml_interface::MlCoordRequest;
use cantrip_ml_interface::MLCOORD_REQUEST_DATA_SIZE;
use cantrip_ml_shared::ImageId;
use cantrip_os_common::camkes;
use cantrip_os_common::logger;
use cantrip_os_common::sel4_sys;
use cantrip_timer_interface::*;
use log::error;
use spin::Mutex;

use camkes::*;
use logger::*;

// Generated code...
mod generated {
    include!(concat!(env!("SEL4_OUT_DIR"), "/../ml_coordinator/camkes.rs"));
}
use generated::*;

static ML_COORD: Mutex<MLCoordinator> = Mutex::new(MLCoordinator::new());

struct MlCoordinatorControlThread;
impl CamkesThreadInterface for MlCoordinatorControlThread {
    fn pre_init() {
        // XXX how to handle "maybe" inclusion
        static_logger!(logger);

        static mut HEAP_MEMORY: [u8; 4 * 1024] = [0; 4 * 1024];
        unsafe {
            CAMKES.pre_init(&mut HEAP_MEMORY);
        }
    }
    fn run() {
        shared_irq_loop!(
            irq,
            host_req => || {
                ML_COORD.lock().handle_host_req_interrupt();
                true
            },
            finish => || {
                ML_COORD.lock().handle_return_interrupt();
                true
            },
            instruction_fault => || {
                ML_COORD.lock().handle_instruction_fault_interrupt();
                true
            },
            data_fault => || {
                ML_COORD.lock().handle_data_fault_interrupt();
                true
            }
        );
    }
}

struct TimerInterfaceThread;
impl CamkesThreadInterface for TimerInterfaceThread {
    fn run() {
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
}

struct MlcoordInterfaceThread;
impl CamkesThreadInterface for MlcoordInterfaceThread {
    fn run() {
        ML_COORD.lock().init();

        rpc_basic_recv!(mlcoord, MLCOORD_REQUEST_DATA_SIZE, MlCoordError::Success);
    }
}
impl MlcoordInterfaceThread {
    fn dispatch(
        client_badge: usize,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> Result<usize, MlCoordError> {
        let _cleanup = Camkes::cleanup_request_cap();
        let request = match postcard::from_bytes::<MlCoordRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(MlCoordError::DeserializeError),
        };

        match request {
            MlCoordRequest::CompletedJobs => Self::completed_jobs_request(reply_buffer),
            MlCoordRequest::GetOutput {
                bundle_id,
                model_id,
            } => Self::get_output_request(bundle_id, model_id, reply_buffer),
            MlCoordRequest::Oneshot {
                bundle_id,
                model_id,
            } => Self::oneshot_request(client_badge, bundle_id, model_id),
            MlCoordRequest::Periodic {
                bundle_id,
                model_id,
                rate_in_ms,
            } => Self::periodic_request(client_badge, bundle_id, model_id, rate_in_ms),
            MlCoordRequest::Cancel {
                bundle_id,
                model_id,
            } => Self::cancel_request(bundle_id, model_id),
            MlCoordRequest::DebugState => Self::debug_state_request(),
            MlCoordRequest::Capscan => Self::capscan_request(),
        }
    }

    fn completed_jobs_request(reply_buffer: &mut [u8]) -> Result<usize, MlCoordError> {
        let job_mask = ML_COORD.lock().completed_jobs();
        let reply_slice = postcard::to_slice(&CompleteJobsResponse { job_mask }, reply_buffer)
            .or(Err(MlCoordError::SerializeError))?;
        Ok(reply_slice.len())
    }

    fn get_output_request(
        bundle_id: &str,
        model_id: &str,
        reply_buffer: &mut [u8],
    ) -> Result<usize, MlCoordError> {
        let image_id = ImageId {
            bundle_id: bundle_id.to_string(),
            model_id: model_id.to_string(),
        };
        let output = ML_COORD.lock().get_output(&image_id)?;
        let reply_slice = postcard::to_slice(&GetOutputResponse { output }, reply_buffer)
            .or(Err(MlCoordError::SerializeError))?;
        Ok(reply_slice.len())
    }

    fn oneshot_request(
        client_badge: usize,
        bundle_id: &str,
        model_id: &str,
    ) -> Result<usize, MlCoordError> {
        let image_id = ImageId {
            bundle_id: bundle_id.to_string(),
            model_id: model_id.to_string(),
        };
        ML_COORD.lock().oneshot(client_badge, image_id)?;
        Ok(0)
    }

    fn periodic_request(
        client_badge: usize,
        bundle_id: &str,
        model_id: &str,
        rate_in_ms: u32,
    ) -> Result<usize, MlCoordError> {
        let image_id = ImageId {
            bundle_id: bundle_id.to_string(),
            model_id: model_id.to_string(),
        };
        ML_COORD
            .lock()
            .periodic(client_badge, image_id, rate_in_ms)?;
        Ok(0)
    }

    fn cancel_request(bundle_id: &str, model_id: &str) -> Result<usize, MlCoordError> {
        let image_id = ImageId {
            bundle_id: bundle_id.to_string(),
            model_id: model_id.to_string(),
        };
        ML_COORD.lock().cancel(&image_id)?;
        Ok(0)
    }

    fn debug_state_request() -> Result<usize, MlCoordError> {
        ML_COORD.lock().debug_state();
        Ok(0)
    }

    fn capscan_request() -> Result<usize, MlCoordError> {
        let _ = Camkes::capscan();
        Ok(0)
    }
}
