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
 *
 * Functions defined here are entrypoints defined by the CAmkES component
 * definition in SDKRuntime.camkes, and bind the C entry points to Rust by
 * calling Rust methods in the SDKRuntimeInterface impl, CANTRIP_SDK.
 *
 * This is the lowest level entry point from C to Rust in CAmkES.
 */

#![no_std]
#![allow(clippy::missing_safety_doc)]

use static_assertions::assert_cfg;
// NB: the RPC implementation uses MCS syscalls
assert_cfg!(feature = "CONFIG_KERNEL_MCS");

extern crate alloc;
use alloc::vec;
use cantrip_memory_interface::cantrip_object_alloc_in_toplevel_static;
use cantrip_memory_interface::ObjDesc;
use cantrip_os_common::camkes::{seL4_CPath, Camkes};
use cantrip_os_common::copyregion::CopyRegion;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use cantrip_sdk_manager::SDKManagerError;
use cantrip_sdk_manager::SDKManagerInterface;
use cantrip_sdk_runtime::CantripSDKRuntime;
use core::mem::size_of;
use core::ptr;
use cstr_core::CStr;
use log::error;
use log::info;

use sdk_interface::SDKAppId;
use sdk_interface::SDKError;
use sdk_interface::SDKRuntimeError;
use sdk_interface::SDKRuntimeInterface;
use sdk_interface::SDKRuntimeRequest;
use sdk_interface::SDKRUNTIME_REQUEST_DATA_SIZE;

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_EndpointObject;
use sel4_sys::seL4_Fault;
use sel4_sys::seL4_MessageInfo;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_Recv;
use sel4_sys::seL4_ReplyObject;
use sel4_sys::seL4_ReplyRecv;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_Word;

const PAGE_SIZE: usize = 1 << seL4_PageBits;

extern "C" {
    static mut SDK_PARAMS: [seL4_Word; PAGE_SIZE / size_of::<seL4_Word>()];
}

static mut CAMKES: Camkes = Camkes::new("SDKRuntime");

// Server RPC plumbing.
static mut CANTRIP_SDK_ENDPOINT: seL4_CPtr = 0;
static mut CANTRIP_SDK_REPLY: seL4_CPtr = 0;
static mut CANTRIP_SDK_RECV_SLOT: seL4_CPtr = 0;

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
            CANTRIP_SDK_ENDPOINT = ep_slot.release();
        }

        // NB: SDKRuntime needs the original (unbadged) cap to mint badged
        // caps with WGP rights for applications (returned by get_endpoint).
        runtime.init(&endpoint);
    }
    runtime
}

/// CAmkES component pre-init method.
///
/// We use this to initialize our Rust heap, logger, etc.
#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static mut HEAP_MEMORY: [u8; 8 * 1024] = [0; 8 * 1024];
    CAMKES.pre_init(log::LevelFilter::Trace, &mut HEAP_MEMORY);

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
    CANTRIP_SDK_REPLY = reply_slot.release();

    // Receive slot for frames with RPC parameters.
    CANTRIP_SDK_RECV_SLOT = CSpaceSlot::new().release();

    cantrip_sdk();
}

fn delete_path(path: &seL4_CPath) -> seL4_Result {
    unsafe { seL4_CNode_Delete(path.0, path.1, path.2 as u8) }
}

/// Server-side of SDKRuntime request processing.  Note CAmkES does not
/// participate in the RPC processing we use the control thread instead
/// of having CAmkES create an interface thread, and pass parameters
/// through a page frame attached to the IPC buffer.
#[no_mangle]
pub unsafe extern "C" fn run() -> ! {
    let recv_path = &Camkes::top_level_path(CANTRIP_SDK_RECV_SLOT);
    CAMKES.init_recv_path(recv_path);
    Camkes::debug_assert_slot_empty("run", recv_path);

    let mut copy_region = CopyRegion::new(ptr::addr_of_mut!(SDK_PARAMS[0]), PAGE_SIZE);

    // Do initial Recv; after this we use ReplyRecv to minimize syscalls.
    let mut sdk_runtime_badge: seL4_Word = 0;
    let mut response: Result<(), SDKError>;
    let mut info = seL4_Recv(
        /*src=*/ CANTRIP_SDK_ENDPOINT,
        /*sender=*/ &mut sdk_runtime_badge as _,
        /*reply=*/ CANTRIP_SDK_REPLY,
    );

    loop {
        // Check for a fault condition and handle those specially.
        if info.get_label() < (SDKRuntimeRequest::Ping as usize) {
            let app_id = sdk_runtime_badge as SDKAppId;
            let label = info.get_label() as usize;
            let fault_type = seL4_Fault::try_from(label);

            // XXX Do something with the fault -- notify ProcessManager about it
            // so we can clean up that whole thread and mess. Should be as
            // simple as calling stop.

            match fault_type {
                Ok(seL4_Fault::seL4_NullFault) => { info!("app {} faulted ({:?}): normal exit or termination.", app_id, fault_type); }
                Ok(seL4_Fault::seL4_CapFault) => { info!("app {} faulted ({:?}): invalid capability usage.", app_id, label); }
                Ok(seL4_Fault::seL4_UnknownSyscall) => { info!("app {} faulted ({:?}): unknown syscall requested.", app_id, label); }
                Ok(seL4_Fault::seL4_UserException) => { info!("app {} faulted ({:?}): user exception requested.", app_id, label); }

                Ok(seL4_Fault::seL4_BogusException) => { error!("Impossible! We received a Bogus Exception! My one weakness! How did you know?!"); }

                #[cfg(feature = "CONFIG_KERNEL_MCS")]
                Ok(seL4_Fault::seL4_Timeout) => { info!("app {} faulted ({:?}): application timed out.", app_id, label); }

                Ok(seL4_Fault::seL4_VMFault) => { info!("app {} faulted ({:?}): virtual-memory fault.", app_id, label); }

                Err(_) => {}
            }

            // Clean up any request caps
            delete_path(recv_path).expect("delete");
            Camkes::debug_assert_slot_empty("run", recv_path);

            // Can't respond to one of these messages, really, since doing so
            // would unsuspend the faulting thread, leading possibly to another
            // fault depending on the type. For now, just wait for another
            // message and start back at the top of the loop.

            // XXX debug seL4 complains about an unexecuted reply cap here.
            info = seL4_Recv(
                /*src=*/ CANTRIP_SDK_ENDPOINT,
                /*sender=*/ &mut sdk_runtime_badge as _,
                /*reply=*/ CANTRIP_SDK_REPLY,
            );

            continue;
        }

        Camkes::debug_assert_slot_frame("run", recv_path);
        // seL4_Recv & seL4_ReplyRecv return any badge but do not reset
        // the ipcbuffer state. If the ipcbuffer is turned around for a
        // send operation the received badge may be interpreted as an
        // outbound capability. To guard against this clear the field here
        // (so it happens for both calls) with clear_request_cap().
        Camkes::clear_request_cap();

        // Map the frame with RPC parameters and process the request.
        if copy_region.map(recv_path.1).is_ok() {
            // The request token is passed in the MessageInfo label field.
            // Any request-specific parameters are serialized in the first
            // half of the page, with the second half reserved for reply data.
            // We might consider sending a request length out-of-band (like
            // the request token) to enable variable page splitting.
            let (request_slice, reply_slice) = copy_region
                .as_mut()
                .split_at_mut(SDKRUNTIME_REQUEST_DATA_SIZE);
            let request_slice = &*request_slice; // NB: immutable alias

            let app_id = sdk_runtime_badge as SDKAppId; // XXX safe?
            response = match SDKRuntimeRequest::try_from(info.get_label()) {
                Ok(SDKRuntimeRequest::Ping) => ping_request(app_id, request_slice, reply_slice),
                Ok(SDKRuntimeRequest::Log) => log_request(app_id, request_slice, reply_slice),
                Ok(SDKRuntimeRequest::ReadKey) => {
                    read_key_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::WriteKey) => {
                    write_key_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::DeleteKey) => {
                    delete_key_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::OneshotTimer) => {
                    timer_oneshot_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::PeriodicTimer) => {
                    timer_periodic_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::CancelTimer) => {
                    timer_cancel_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::WaitForTimers) => {
                    timer_wait_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::PollForTimers) => {
                    timer_poll_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::OneshotModel) => {
                    model_oneshot_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::PeriodicModel) => {
                    model_periodic_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::CancelModel) => {
                    model_cancel_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::WaitForModel) => {
                    model_wait_request(app_id, request_slice, reply_slice)
                }
                Ok(SDKRuntimeRequest::PollForModels) => {
                    model_poll_request(app_id, request_slice, reply_slice)
                }
                Err(_) => {
                    // TODO(b/254286176): possible ddos
                    error!("Unknown RPC request {}", info.get_label());
                    Err(SDKError::UnknownRequest)
                }
            };
            copy_region.unmap().expect("unmap");
        } else {
            // TODO(b/254286176): possible ddos
            error!("Unable to map RPC parameters; badge {}", sdk_runtime_badge);
            response = Err(SDKError::MapPageFailed);
        }

        delete_path(recv_path).expect("delete");
        Camkes::debug_assert_slot_empty("run", recv_path);

        info = seL4_ReplyRecv(
            /*src=*/ CANTRIP_SDK_ENDPOINT,
            /*msgInfo=*/
            seL4_MessageInfo::new(
                /*label=*/ SDKRuntimeError::from(response) as seL4_Word,
                /*capsUnwrapped=*/ 0,
                /*extraCaps=*/ 0,
                /*length=*/ 0,
            ),
            /*sender=*/ &mut sdk_runtime_badge as _,
            /*reply=*/ CANTRIP_SDK_REPLY,
        );
    }
}

// SDK RPC request handling: unmarshal request, dispatch to CANTRIP_SDK,
// and marshal reply.

fn serialize_failure(e: postcard::Error) -> SDKError {
    error!("serialize failed: {:?}", e);
    SDKError::SerializeFailed
}
fn deserialize_failure(e: postcard::Error) -> SDKError {
    error!("deserialize failed: {:?}", e);
    SDKError::DeserializeFailed
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

// SDKManager RPC handling; these arrive via CAmkES so have a C linkage.

#[no_mangle]
pub unsafe extern "C" fn sdk_manager_get_endpoint(
    c_app_id: *const cstr_core::c_char,
) -> SDKManagerError {
    let ret_status = match CStr::from_ptr(c_app_id).to_str() {
        Ok(app_id) => match cantrip_sdk().get_endpoint(app_id) {
            Ok(cap_endpoint) => {
                Camkes::set_reply_cap_release(cap_endpoint);
                SDKManagerError::SmSuccess
            }
            Err(e) => e,
        },
        Err(_) => SDKManagerError::SmAppIdInvalid,
    };
    ret_status
}

#[no_mangle]
pub unsafe extern "C" fn sdk_manager_release_endpoint(
    c_app_id: *const cstr_core::c_char,
) -> SDKManagerError {
    let ret_status = match CStr::from_ptr(c_app_id).to_str() {
        Ok(app_id) => match cantrip_sdk().release_endpoint(app_id) {
            Ok(_) => SDKManagerError::SmSuccess,
            Err(e) => e,
        },
        Err(_) => SDKManagerError::SmAppIdInvalid,
    };
    ret_status
}

#[no_mangle]
pub unsafe extern "C" fn sdk_manager_capscan() { let _ = Camkes::capscan(); }
