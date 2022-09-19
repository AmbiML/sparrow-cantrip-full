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
use core::mem::size_of;
use core::ptr;
use cstr_core::CStr;
use cantrip_memory_interface::cantrip_object_alloc_in_toplevel;
use cantrip_memory_interface::ObjDesc;
use cantrip_os_common::camkes::{seL4_CPath, Camkes};
use cantrip_os_common::copyregion::CopyRegion;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use cantrip_sdk_interface::KeyValueData;
use cantrip_sdk_interface::SDKAppId;
use cantrip_sdk_interface::SDKError;
use cantrip_sdk_interface::SDKReplyHeader;
use cantrip_sdk_interface::SDKRuntimeError;
use cantrip_sdk_interface::SDKRuntimeInterface;
use cantrip_sdk_interface::SDKRuntimeRequest;
use cantrip_sdk_interface::SDKRUNTIME_REQUEST_DATA_SIZE;
use cantrip_sdk_manager::SDKManagerError;
use cantrip_sdk_manager::SDKManagerInterface;
use cantrip_sdk_runtime::CantripSDKRuntime;
use log::error;

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_EndpointObject;
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
// NB: CANTRIP_SDK cannot be used before setup is completed with a call to init()
static mut CANTRIP_SDK: CantripSDKRuntime = CantripSDKRuntime::empty();

// Server RPC plumbing.
static mut CANTRIP_SDK_ENDPOINT: seL4_CPtr = 0;
static mut CANTRIP_SDK_REPLY: seL4_CPtr = 0;
static mut CANTRIP_SDK_RECV_SLOT: seL4_CPtr = 0;

/// CAmkES component pre-init method.
///
/// We use this to initialize our Rust heap, logger, etc.
#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static mut HEAP_MEMORY: [u8; 8 * 1024] = [0; 8 * 1024];
    CAMKES.pre_init(log::LevelFilter::Trace, &mut HEAP_MEMORY);

    // Setup the SDKRuntime service from scratch (no CAmkES help).
    let bundle = cantrip_object_alloc_in_toplevel(vec![
        ObjDesc::new(seL4_EndpointObject, 1, 0),
        ObjDesc::new(seL4_ReplyObject, 1, 1),
    ])
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
    CANTRIP_SDK_ENDPOINT = ep_slot.release();

    // Create reply (WG).
    let reply = Camkes::top_level_path(bundle.objs[1].cptr);
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

    // NB: SDKRuntime needs the original (unbadged) cap to mint badged
    // caps with WGP rights for applications (returned by get_endpoint).
    CANTRIP_SDK.init(&endpoint);
}

fn delete_path(path: &seL4_CPath) -> seL4_Result {
    unsafe { seL4_CNode_Delete(path.0, path.1, path.2 as u8) }
}

fn reply_error(error: SDKError, reply_slice: &mut [u8]) {
    // XXX check return
    let _ = postcard::to_slice(
        &SDKReplyHeader {
            status: error.into(),
        },
        reply_slice,
    );
}

/// Server-side of SDKRuntime request processing.  Note CAmkES does not
/// participate in the RPC processing we use the control thread instead
/// of having CAmkES create an interface thread and pass parameters through
/// a page frame attached to the IPC buffer.
#[no_mangle]
pub unsafe extern "C" fn run() -> ! {
    let recv_path = &Camkes::top_level_path(CANTRIP_SDK_RECV_SLOT);
    CAMKES.init_recv_path(recv_path);
    Camkes::debug_assert_slot_empty("run", recv_path);

    let mut copy_region = CopyRegion::new(ptr::addr_of_mut!(SDK_PARAMS[0]), PAGE_SIZE);

    // Do initial Recv; after this we use ReplyRecv to minimize syscalls.
    let mut sdk_runtime_badge: seL4_Word = 0;
    seL4_Recv(CANTRIP_SDK_ENDPOINT, &mut sdk_runtime_badge as _, CANTRIP_SDK_REPLY);
    loop {
        Camkes::debug_assert_slot_frame("run", recv_path);
        // seL4_Recv & seL4_ReplyRecv return any badge but do not reset
        // the ipcbuffer state. If the ipcbuffer is turned around for a
        // send operation the received badge may be interpreted as an
        // outbound capability. To guard against this clear the field here
        // (so it happens for both calls) with clear_request_cap().
        Camkes::clear_request_cap();
        // Map the frame with RPC parameters and decode the request header.
        if copy_region.map(recv_path.1).is_ok() {
            // The client serializes an SDKRequestHeader first with the
            // request id. This is followed by request-specific arguments
            // that must be processed by each handler.
            let (request_slice, reply_slice) = copy_region
                .as_mut()
                .split_at_mut(SDKRUNTIME_REQUEST_DATA_SIZE);
            let request_slice = &*request_slice; // NB: immutable alias
            match postcard::take_from_bytes::<cantrip_sdk_interface::SDKRequestHeader>(request_slice) {
                Ok((header, args_slice)) => {
                    let app_id = sdk_runtime_badge as SDKAppId; // XXX safe?
                    if let Err(status) = match header.request {
                        SDKRuntimeRequest::Ping => ping_request(app_id, args_slice, reply_slice),
                        SDKRuntimeRequest::Log => log_request(app_id, args_slice, reply_slice),
                        SDKRuntimeRequest::ReadKey => {
                            read_key_request(app_id, args_slice, reply_slice)
                        }
                        SDKRuntimeRequest::WriteKey => {
                            write_key_request(app_id, args_slice, reply_slice)
                        }
                        SDKRuntimeRequest::DeleteKey => {
                            delete_key_request(app_id, args_slice, reply_slice)
                        }
                    } {
                        reply_error(status, reply_slice);
                    }
                }
                Err(err) => reply_error(deserialize_failure(err), reply_slice),
            }
            copy_region.unmap().expect("unmap");
        } else {
            error!("Unable to map RPC parameters; badge {}", sdk_runtime_badge);
            // TODO(jtgans): no way to return an error; signal ProcessManager to stop app?
        }
        delete_path(recv_path).expect("delete");
        Camkes::debug_assert_slot_empty("run", recv_path);

        let info = seL4_MessageInfo::new(0, 0, 0, /*length=*/ 0);
        seL4_ReplyRecv(CANTRIP_SDK_ENDPOINT, info, &mut sdk_runtime_badge as _, CANTRIP_SDK_REPLY);
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
    unsafe { CANTRIP_SDK.ping(app_id) }
}

fn log_request(
    app_id: SDKAppId,
    request_slice: &[u8],
    _reply_slice: &mut [u8],
) -> Result<(), SDKError> {
    let request = postcard::from_bytes::<cantrip_sdk_interface::LogRequest>(request_slice)
        .map_err(deserialize_failure)?;
    let msg = core::str::from_utf8(request.msg).map_err(|_| SDKError::InvalidString)?;
    unsafe { CANTRIP_SDK.log(app_id, msg) }
}

fn read_key_request(
    app_id: SDKAppId,
    request_slice: &[u8],
    reply_slice: &mut [u8],
) -> Result<(), SDKError> {
    let request = postcard::from_bytes::<cantrip_sdk_interface::ReadKeyRequest>(request_slice)
        .map_err(deserialize_failure)?;
    #[allow(clippy::uninit_assumed_init)]
    let mut keyval: KeyValueData = unsafe { ::core::mem::MaybeUninit::uninit().assume_init() };
    let value = unsafe { CANTRIP_SDK.read_key(app_id, request.key, &mut keyval)? };
    let _ = postcard::to_slice(
        &cantrip_sdk_interface::ReadKeyResponse {
            header: SDKReplyHeader::new(SDKRuntimeError::SDKSuccess),
            value,
        },
        reply_slice,
    )
    .map_err(serialize_failure)?;
    Ok(())
}

fn write_key_request(
    app_id: SDKAppId,
    request_slice: &[u8],
    _reply_slice: &mut [u8],
) -> Result<(), SDKError> {
    let request = postcard::from_bytes::<cantrip_sdk_interface::WriteKeyRequest>(request_slice)
        .map_err(deserialize_failure)?;
    // NB: the serialized data are variable length so copy to convert
    let mut keyval = [0u8; cantrip_sdk_interface::KEY_VALUE_DATA_SIZE];
    keyval[..request.value.len()].copy_from_slice(request.value);
    unsafe { CANTRIP_SDK.write_key(app_id, request.key, &keyval) }
}

fn delete_key_request(
    app_id: SDKAppId,
    request_slice: &[u8],
    _reply_slice: &mut [u8],
) -> Result<(), SDKError> {
    let request = postcard::from_bytes::<cantrip_sdk_interface::DeleteKeyRequest>(request_slice)
        .map_err(deserialize_failure)?;
    unsafe { CANTRIP_SDK.delete_key(app_id, request.key) }
}

// SDKManager RPC handling; these arrive via CAmkES so have a C linkage.

#[no_mangle]
pub unsafe extern "C" fn sdk_manager_get_endpoint(
    c_app_id: *const cstr_core::c_char,
) -> SDKManagerError {
    let ret_status = match CStr::from_ptr(c_app_id).to_str() {
        Ok(app_id) => match CANTRIP_SDK.get_endpoint(app_id) {
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
        Ok(app_id) => match CANTRIP_SDK.release_endpoint(app_id) {
            Ok(_) => SDKManagerError::SmSuccess,
            Err(e) => e,
        },
        Err(_) => SDKManagerError::SmAppIdInvalid,
    };
    ret_status
}

#[no_mangle]
pub unsafe extern "C" fn sdk_manager_capscan() { let _ = Camkes::capscan(); }
