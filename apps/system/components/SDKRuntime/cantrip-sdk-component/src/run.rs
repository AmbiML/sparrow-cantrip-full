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

/*!
 * CantripOS SDK Manager CAmkES component support routines.
 */

#![no_std]
//error[E0658]: dereferencing raw mutable pointers in statics is unstable
#![feature(const_mut_refs)]

use static_assertions::assert_cfg;
// NB: the RPC implementation uses MCS syscalls
assert_cfg!(feature = "CONFIG_KERNEL_MCS");

extern crate alloc;
use alloc::string::String;
use alloc::vec;
use cantrip_memory_interface::cantrip_object_alloc_in_toplevel_static;
use cantrip_memory_interface::ObjDesc;
use cantrip_os_common::camkes;
use cantrip_os_common::copyregion::CopyRegion;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::logger;
use cantrip_os_common::sel4_sys;
use cantrip_sdk_manager::SDKManagerError;
use cantrip_sdk_manager::SDKManagerInterface;
use cantrip_sdk_manager::SDKManagerRequest;
use cantrip_sdk_manager::SDK_MANAGER_REQUEST_DATA_SIZE;
use cantrip_sdk_runtime::CantripSDKRuntime;
use log::{error, info};

use camkes::*;
use logger::*;

use sdk_interface::SDKAppId;
use sdk_interface::SDKError;
use sdk_interface::SDKRuntimeError;
use sdk_interface::SDKRuntimeInterface;
use sdk_interface::SDKRuntimeRequest;
use sdk_interface::SDKRUNTIME_REQUEST_DATA_SIZE;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_EndpointObject;
use sel4_sys::seL4_FaultTag;
use sel4_sys::seL4_MessageInfo;
use sel4_sys::seL4_Recv;
use sel4_sys::seL4_ReplyObject;
use sel4_sys::seL4_ReplyRecv;
use sel4_sys::seL4_Word;

// Generated code...
mod generated {
    include!(concat!(env!("SEL4_OUT_DIR"), "/../sdk_runtime/camkes.rs"));
}
use generated::*;

fn cantrip_sdk() -> impl SDKManagerInterface + SDKRuntimeInterface {
    static CANTRIP_SDK: CantripSDKRuntime = CantripSDKRuntime::empty();
    let mut runtime = CANTRIP_SDK.get();
    if runtime.is_empty() {
        // Setup the SDKRuntime service (endpoint part) from scratch (no CAmkES help).
        let bundle =
            cantrip_object_alloc_in_toplevel_static(vec![ObjDesc::new(seL4_EndpointObject, 1, 0)])
                .expect("alloc");

        // Create endpoint (R)
        let endpoint = Camkes::top_level_path(bundle.objs[0].cptr);
        let mut ep_slot = CSpaceSlot::new();
        ep_slot
            .copy_to(
                endpoint.0,
                endpoint.1,
                endpoint.2 as u8,
                seL4_CapRights::new(
                    /*grant_reply=*/ 0, /*grant=*/ 0, /*read=*/ 1, /*write=*/ 0,
                ),
            )
            .expect("endpoint");
        unsafe {
            SDKRUNTIME_ENDPOINT = ep_slot.release();
        }

        // NB: SDKRuntime needs the original (unbadged) cap to mint badged
        // caps with WGP rights for applications (returned by get_endpoint).
        runtime.init(&endpoint);
    }
    runtime
}

// Server RPC plumbing.
static mut SDKRUNTIME_ENDPOINT: seL4_CPtr = 0;
static mut SDKRUNTIME_REPLY: seL4_CPtr = 0;
static mut SDKRUNTIME_RECV_SLOT: seL4_CPtr = 0;

struct SdkRuntimeControlThread;
impl CamkesThreadInterface for SdkRuntimeControlThread {
    fn pre_init() {
        // XXX how to handle "maybe" inclusion
        static_logger!(logger);

        static mut HEAP_MEMORY: [u8; 8 * 1024] = [0; 8 * 1024];
        unsafe {
            CAMKES.pre_init(&mut HEAP_MEMORY);
        }

        // Setup the SDKRuntime service (reply part) from scratch (no CAmkES help).
        // NB: the endpoint part is done in cantrip_sdk().
        let bundle =
            cantrip_object_alloc_in_toplevel_static(vec![ObjDesc::new(seL4_ReplyObject, 1, 1)])
                .expect("alloc");

        // Create reply (WG).
        let reply = Camkes::top_level_path(bundle.objs[0].cptr);
        let mut reply_slot = CSpaceSlot::new();
        reply_slot
            .copy_to(
                reply.0,
                reply.1,
                reply.2 as u8,
                seL4_CapRights::new(
                    /*grant_reply=*/ 0, /*grant=*/ 1, // XXX not sending back caps
                    /*read=*/ 0, /*write=*/ 1,
                ),
            )
            .expect("reply");
        // NB: hold onto reply for now (only need/usee the WG copy)
        unsafe {
            SDKRUNTIME_REPLY = reply_slot.release();
        }

        // Receive slot for frames with RPC parameters.
        unsafe {
            SDKRUNTIME_RECV_SLOT = CSpaceSlot::new().release();
        }

        cantrip_sdk();
    }

    // Server-side of SDKRuntime request processing.  Note CAmkES does not
    // participate in the RPC processing we use the control thread instead
    // of having CAmkES create an interface thread, and pass parameters
    // through a page frame attached to the IPC buffer.
    fn run() {
        let recv_path = &Camkes::top_level_path(unsafe { SDKRUNTIME_RECV_SLOT });
        CAMKES.init_recv_path(recv_path);
        Camkes::debug_assert_slot_empty("run", recv_path);

        let mut copy_region = unsafe { CopyRegion::new(get_sdk_params_mut()) };

        // Do initial Recv; after this we use ReplyRecv to minimize syscalls.
        unsafe {
            let mut client_badge: seL4_Word = 0;
            let mut response: Result<(), SDKError>;
            let mut info = seL4_Recv(
                /*src=*/ SDKRUNTIME_ENDPOINT,
                /*sender=*/ &mut client_badge as _,
                /*reply=*/ SDKRUNTIME_REPLY,
            );
            loop {
                let label = info.get_label();
                let app_id = client_badge as SDKAppId; // XXX safe?

                // Check for a fault condition and handle those specially.
                if label < (SDKRuntimeRequest::Ping as usize) {
                    match seL4_FaultTag::try_from(label) {
                        Ok(fault_tag) => {
                            #[cfg(feature = "CONFIG_DEBUG_BUILD")]
                            print_fault_debug(app_id, fault_tag);

                            #[cfg(not(feature = "CONFIG_DEBUG_BUILD"))]
                            info!("Fault tag {} from {}", fault_tag as usize, app_id);
                        }
                        Err(_) => error!("Bad fault tag {} on msg from {}", label, app_id),
                    }

                    Camkes::debug_assert_slot_empty("fault", recv_path);

                    // Can't respond to one of these messages, really, since doing so
                    // would unsuspend the faulting thread, leading possibly to another
                    // fault depending on the type. For now, just wait for another
                    // message and start back at the top of the loop.

                    // XXX debug seL4 complains about an unexecuted reply cap here.
                    info = seL4_Recv(
                        /*src=*/ SDKRUNTIME_ENDPOINT,
                        /*sender=*/ &mut client_badge as _,
                        /*reply=*/ SDKRUNTIME_REPLY,
                    );
                    continue;
                }

                Camkes::debug_assert_slot_frame("run", recv_path);
                // seL4_Recv & seL4_ReplyRecv return any badge but do not reset
                // the ipcbuffer state. If the ipcbuffer is turned around for a
                // send operation the received badge may be interpreted as an
                // outbound capability. To guard against this clear the field here
                // (so it happens for both calls) with clear_request_cap().
                // XXX not true with rust templates
                Camkes::clear_request_cap();
                // Map the frame with RPC parameters and process the request.
                if copy_region.map(recv_path.1).is_ok() {
                    // The request token is passed in the MessageInfo label field.
                    // Any request-specific parameters are serialized in the first
                    // half of the page, with the second half reserved for reply data.
                    // We might consider sending a request length out-of-band (like
                    // the request token) to enable variable page splitting.
                    //
                    // NB: the request_slice is immutable over the processing
                    //   below so it's safe to pass (deserialized) values to
                    //   the implementation(s) below.
                    let (request_slice, reply_slice) = copy_region
                        .as_mut()
                        .split_at_mut(SDKRUNTIME_REQUEST_DATA_SIZE);
                    let request_slice = &*request_slice; // NB: immutable alias

                    // TODO(sleffler): decode from shared page instead of label
                    response = match SDKRuntimeRequest::try_from(label) {
                        Ok(tag) => Self::request(tag, app_id, request_slice, reply_slice),
                        Err(_) => {
                            // TODO(b/254286176): possible ddos
                            error!("Unknown RPC request {}", label);
                            Err(SDKError::UnknownRequest)
                        }
                    };
                    copy_region.unmap().expect("unmap");
                } else {
                    // TODO(b/254286176): possible ddos
                    error!("Unable to map RPC parameters; badge {}", client_badge);
                    response = Err(SDKError::MapPageFailed);
                }
                Camkes::delete_path(recv_path).expect("delete");
                Camkes::debug_assert_slot_empty("run", recv_path);

                info = seL4_ReplyRecv(
                    /*src=*/ SDKRUNTIME_ENDPOINT,
                    /*msgInfo=*/
                    seL4_MessageInfo::new(
                        /*label=*/ SDKRuntimeError::from(response) as seL4_Word,
                        /*capsUnwrapped=*/ 0,
                        /*extraCaps=*/ 0,
                        /*length=*/ 0,
                    ),
                    /*sender=*/ &mut client_badge as _,
                    /*reply=*/ SDKRUNTIME_REPLY,
                );
            }
        }
    }
}

#[cfg(feature = "CONFIG_DEBUG_BUILD")]
fn print_fault_debug(app_id: SDKAppId, fault_type: seL4_FaultTag) {
    use sel4_sys::seL4_GetMR;
    match fault_type {
        seL4_FaultTag::seL4_Fault_NullFault => {
            let _ = cantrip_sdk().log(app_id, "normal exit or termination");
        }
        seL4_FaultTag::seL4_Fault_CapFault => {
            let _ = cantrip_sdk().log(app_id, "invalid capability");
        }
        seL4_FaultTag::seL4_Fault_UnknownSyscall => {
            let _ = cantrip_sdk().log(app_id, "unknown syscall");
        }
        seL4_FaultTag::seL4_Fault_UserException => {
            let _ = cantrip_sdk().log(app_id, "user exception");
        }
        seL4_FaultTag::seL4_Fault_VMFault => {
            let _ = cantrip_sdk().log(app_id, "virtual-memory fault:");
            info!(target: "", "IP       {:#010x}", unsafe { seL4_GetMR(0) });
            info!(target: "", "Addr     {:#010x}", unsafe { seL4_GetMR(1) });
            info!(target: "", "Prefetch {:#x}", unsafe { seL4_GetMR(2) });
            info!(target: "", "FSR      {:#x}", unsafe { seL4_GetMR(3) });
            info!(target: "", "Length   {:#x}", unsafe { seL4_GetMR(4) });
        }

        #[cfg(feature = "CONFIG_KERNEL_MCS")]
        seL4_FaultTag::seL4_Fault_Timeout => {
            let _ = cantrip_sdk().log(app_id, "application timed out");
        }
    }
}

fn serialize_failure(e: postcard::Error) -> SDKError {
    error!("serialize failed: {:?}", e);
    SDKError::SerializeFailed
}
fn deserialize_failure(e: postcard::Error) -> SDKError {
    error!("deserialize failed: {:?}", e);
    SDKError::DeserializeFailed
}

impl SdkRuntimeControlThread {
    fn request(
        request: SDKRuntimeRequest,
        app_id: SDKAppId,
        request_slice: &[u8],
        reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        match request {
            SDKRuntimeRequest::Ping => Self::ping_request(app_id, request_slice, reply_slice),
            SDKRuntimeRequest::Log => Self::log_request(app_id, request_slice, reply_slice),
            SDKRuntimeRequest::ReadKey => {
                Self::read_key_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::WriteKey => {
                Self::write_key_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::DeleteKey => {
                Self::delete_key_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::OneshotTimer => {
                Self::timer_oneshot_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::PeriodicTimer => {
                Self::timer_periodic_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::CancelTimer => {
                Self::timer_cancel_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::WaitForTimers => {
                Self::timer_wait_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::PollForTimers => {
                Self::timer_poll_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::OneshotModel => {
                Self::model_oneshot_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::PeriodicModel => {
                Self::model_periodic_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::CancelModel => {
                Self::model_cancel_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::WaitForModel => {
                Self::model_wait_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::PollForModels => {
                Self::model_poll_request(app_id, request_slice, reply_slice)
            }
            SDKRuntimeRequest::GetModelOutput => {
                Self::model_output_request(app_id, request_slice, reply_slice)
            }
        }
    }

    fn ping_request(
        app_id: SDKAppId,
        _request_slice: &[u8],
        _reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        cantrip_sdk().ping(app_id)
    }

    fn log_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        _reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::LogRequest>(request_slice)
            .map_err(deserialize_failure)?;
        let msg = core::str::from_utf8(request.msg).or(Err(SDKError::InvalidString))?;
        cantrip_sdk().log(app_id, msg)
    }

    fn read_key_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::ReadKeyRequest>(request_slice)
            .map_err(deserialize_failure)?;
        let value = cantrip_sdk().read_key(app_id, request.key)?;
        let _ = postcard::to_slice(&sdk_interface::ReadKeyResponse { value: &value }, reply_slice)
            .map_err(serialize_failure)?;
        Ok(())
    }

    fn write_key_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        _reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::WriteKeyRequest>(request_slice)
            .map_err(deserialize_failure)?;
        // NB: the serialized data are variable length so copy to convert
        let mut keyval = [0u8; sdk_interface::KEY_VALUE_DATA_SIZE];
        keyval[..request.value.len()].copy_from_slice(request.value);
        cantrip_sdk().write_key(app_id, request.key, &keyval)
    }

    fn delete_key_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        _reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::DeleteKeyRequest>(request_slice)
            .map_err(deserialize_failure)?;
        cantrip_sdk().delete_key(app_id, request.key)
    }

    fn timer_oneshot_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        _reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::TimerStartRequest>(request_slice)
            .map_err(deserialize_failure)?;
        cantrip_sdk().timer_oneshot(app_id, request.id, request.duration_ms)
    }

    fn timer_periodic_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        _reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::TimerStartRequest>(request_slice)
            .map_err(deserialize_failure)?;
        cantrip_sdk().timer_periodic(app_id, request.id, request.duration_ms)
    }

    fn timer_cancel_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        _reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::TimerCancelRequest>(request_slice)
            .map_err(deserialize_failure)?;
        cantrip_sdk().timer_cancel(app_id, request.id)
    }

    fn timer_wait_request(
        app_id: SDKAppId,
        _request_slice: &[u8],
        reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let mask = cantrip_sdk().timer_wait(app_id)?;
        let _ = postcard::to_slice(&sdk_interface::TimerWaitResponse { mask }, reply_slice)
            .map_err(serialize_failure)?;
        Ok(())
    }

    fn timer_poll_request(
        app_id: SDKAppId,
        _request_slice: &[u8],
        reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let mask = cantrip_sdk().timer_poll(app_id)?;
        let _ = postcard::to_slice(&sdk_interface::TimerWaitResponse { mask }, reply_slice)
            .map_err(serialize_failure)?;
        Ok(())
    }

    fn model_oneshot_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::ModelOneshotRequest>(request_slice)
            .map_err(deserialize_failure)?;
        let id = cantrip_sdk().model_oneshot(app_id, request.model_id)?;
        let _ = postcard::to_slice(&sdk_interface::ModelStartResponse { id }, reply_slice)
            .map_err(serialize_failure)?;
        Ok(())
    }

    fn model_periodic_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::ModelPeriodicRequest>(request_slice)
            .map_err(deserialize_failure)?;
        let id = cantrip_sdk().model_periodic(app_id, request.model_id, request.duration_ms)?;
        let _ = postcard::to_slice(&sdk_interface::ModelStartResponse { id }, reply_slice)
            .map_err(serialize_failure)?;
        Ok(())
    }

    fn model_cancel_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        _reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::ModelCancelRequest>(request_slice)
            .map_err(deserialize_failure)?;
        cantrip_sdk().model_cancel(app_id, request.id)
    }

    fn model_wait_request(
        app_id: SDKAppId,
        _request_slice: &[u8],
        reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let mask = cantrip_sdk().model_wait(app_id)?;
        let _ = postcard::to_slice(&sdk_interface::ModelWaitResponse { mask }, reply_slice)
            .map_err(serialize_failure)?;
        Ok(())
    }

    fn model_poll_request(
        app_id: SDKAppId,
        _request_slice: &[u8],
        reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let mask = cantrip_sdk().model_poll(app_id)?;
        let _ = postcard::to_slice(&sdk_interface::ModelWaitResponse { mask }, reply_slice)
            .map_err(serialize_failure)?;
        Ok(())
    }

    fn model_output_request(
        app_id: SDKAppId,
        request_slice: &[u8],
        reply_slice: &mut [u8],
    ) -> Result<(), SDKError> {
        let request = postcard::from_bytes::<sdk_interface::ModelOutputRequest>(request_slice)
            .map_err(deserialize_failure)?;
        let mloutput = cantrip_sdk().model_output(app_id, request.id)?;
        let _ = postcard::to_slice(
            &sdk_interface::ModelOutputResponse {
                output: sdk_interface::ModelOutput {
                    jobnum: mloutput.jobnum,
                    return_code: mloutput.return_code,
                    epc: mloutput.epc,
                    data: mloutput.data,
                },
            },
            reply_slice,
        )
        .map_err(serialize_failure)?;
        Ok(())
    }
}

type SDKManagerResult = Result<(usize, Option<seL4_CPtr>), SDKManagerError>;

struct SdkManagerInterfaceThread;
impl CamkesThreadInterface for SdkManagerInterfaceThread {
    fn run() {
        // NB: no inbound caps, only (potentially) attached to a reply
        rpc_basic_recv_with_reply_cap!(
            sdk_manager,
            SDK_MANAGER_REQUEST_DATA_SIZE,
            SDKManagerError::Success
        );
    }
}
impl SdkManagerInterfaceThread {
    fn dispatch(
        _client_badge: usize,
        request_buffer: &[u8],
        _reply_buffer: &mut [u8],
    ) -> SDKManagerResult {
        let request = match postcard::from_bytes::<SDKManagerRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(SDKManagerError::DeserializeFailed),
        };
        match request {
            SDKManagerRequest::GetEndpoint(app_id) => Self::get_endpoint_request(app_id),
            SDKManagerRequest::ReleaseEndpoint(app_id) => Self::release_endpoint_request(app_id),
            SDKManagerRequest::Capscan => Self::capscan_request(),
        }
    }
    fn get_endpoint_request(app_id: &str) -> SDKManagerResult {
        // TODO(283265795): copy app_id from the IPCBuffer
        cantrip_sdk()
            .get_endpoint(&String::from(app_id))
            .map(|cap| (0, Some(cap)))
    }
    fn release_endpoint_request(app_id: &str) -> SDKManagerResult {
        // TODO(283265795): copy app_id from the IPCBuffer
        cantrip_sdk()
            .release_endpoint(&String::from(app_id))
            .map(|_| (0, None))
    }
    fn capscan_request() -> SDKManagerResult {
        let _ = Camkes::capscan();
        Ok((0, None))
    }
}
