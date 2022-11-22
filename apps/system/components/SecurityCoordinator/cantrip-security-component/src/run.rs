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

//! Cantrip OS Security Coordinator component support.

// Code here binds the camkes component to the rust code.
#![no_std]
#![allow(clippy::missing_safety_doc)]

use core::slice;
use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use cantrip_security_coordinator::CANTRIP_SECURITY;
use cantrip_security_interface::*;
use log::trace;

use SecurityRequestError::*;

use sel4_sys::seL4_CPtr;

static mut CAMKES: Camkes = Camkes::new("SecurityCoordinator");
static mut SECURITY_RECV_SLOT: seL4_CPtr = 0;

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static mut HEAP_MEMORY: [u8; 8 * 1024] = [0; 8 * 1024];
    // NB: set to max; the LoggerInterface will filter
    CAMKES.pre_init(log::LevelFilter::Trace, &mut HEAP_MEMORY);

    // Complete CANTRIP_SECURITY setup after Global allocator is setup.
    CANTRIP_SECURITY.init();

    SECURITY_RECV_SLOT = CSpaceSlot::new().release();
}

#[no_mangle]
pub unsafe extern "C" fn security__init() {
    // Point the receive path to the well-known empty slot. This will be
    // used to receive CNode's from clients for install requests.
    //
    // NB: this must be done here (rather than someplace like pre_init)
    // so it's in the context of the SecurityCoordinatorInterface thread
    // (so we write the correct ipc buffer).
    let path = &Camkes::top_level_path(SECURITY_RECV_SLOT);
    CAMKES.init_recv_path(path);
    Camkes::debug_assert_slot_empty("security__init", path);
}

fn serialize_failure(e: postcard::Error) -> SecurityRequestError {
    trace!("serialize failed: {:?}", e);
    SreBundleDataInvalid
}
fn deserialize_failure(e: postcard::Error) -> SecurityRequestError {
    trace!("deserialize failed: {:?}", e);
    SreDeserializeFailed
}

fn echo_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let request =
        postcard::from_bytes::<EchoRequest>(request_buffer).map_err(deserialize_failure)?;

    trace!("ECHO {:?}", request.value);
    // NB: cheat, bypass serde
    reply_buffer[0..request.value.len()].copy_from_slice(request.value);
    Ok(())
}

fn install_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let recv_path = unsafe { CAMKES.get_current_recv_path() };
    Camkes::debug_assert_slot_cnode("install_request", &recv_path);

    let mut request =
        postcard::from_bytes::<InstallRequest>(request_buffer).map_err(deserialize_failure)?; // XXX clear_path

    // Move the container CNode so it's not clobbered.
    let mut container_slot = CSpaceSlot::new();
    container_slot
        .move_to(recv_path.0, recv_path.1, recv_path.2 as u8)
        .map_err(|_| SecurityRequestError::SreCapMoveFailed)?; // XXX expect?
    request.set_container_cap(container_slot.release());

    let bundle_id = unsafe { CANTRIP_SECURITY.install(&request.pkg_contents) }?;
    let _ = postcard::to_slice(
        &InstallResponse {
            bundle_id: &bundle_id,
        },
        reply_buffer,
    )
    .map_err(serialize_failure)?;
    Ok(())
}

fn uninstall_request(
    request_buffer: &[u8],
    _reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let request =
        postcard::from_bytes::<UninstallRequest>(request_buffer).map_err(deserialize_failure)?;

    trace!("UNINSTALL {}", request.bundle_id);
    unsafe { CANTRIP_SECURITY.uninstall(request.bundle_id) }
}

fn size_buffer_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let request =
        postcard::from_bytes::<SizeBufferRequest>(request_buffer).map_err(deserialize_failure)?;

    trace!("SIZE BUFFER bundle_id {}", request.bundle_id);
    let buffer_size = unsafe { CANTRIP_SECURITY.size_buffer(request.bundle_id) }?;
    let _ = postcard::to_slice(&SizeBufferResponse { buffer_size }, reply_buffer)
        .map_err(serialize_failure)?;
    Ok(())
}

fn get_manifest_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let request =
        postcard::from_bytes::<GetManifestRequest>(request_buffer).map_err(deserialize_failure)?;

    trace!("GET MANIFEST bundle_id {}", request.bundle_id);
    let manifest = unsafe { CANTRIP_SECURITY.get_manifest(request.bundle_id) }?;
    let _ = postcard::to_slice(
        &GetManifestResponse {
            manifest: &manifest,
        },
        reply_buffer,
    )
    .map_err(serialize_failure)?;
    Ok(())
}

fn load_application_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let request = postcard::from_bytes::<LoadApplicationRequest>(request_buffer)
        .map_err(deserialize_failure)?;

    trace!("LOAD APPLICATION bundle_id {}", request.bundle_id);
    let bundle_frames = unsafe { CANTRIP_SECURITY.load_application(request.bundle_id) }?;
    // TODO(sleffler): maybe rearrange to eliminate clone
    postcard::to_slice(
        &LoadApplicationResponse {
            bundle_frames: bundle_frames.clone(),
        },
        reply_buffer,
    )
    .map_err(serialize_failure)?;
    trace!("LOAD APPLICATION -> {}", bundle_frames);
    // Cleanup allocated slot & mark cap for release after reply completes.
    Camkes::set_reply_cap_release(bundle_frames.cnode);
    Ok(())
}

fn load_model_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let request =
        postcard::from_bytes::<LoadModelRequest>(request_buffer).map_err(deserialize_failure)?;

    let model_frames = unsafe { CANTRIP_SECURITY.load_model(request.bundle_id, request.model_id) }?;
    // TODO(sleffler): maybe rearrange to eliminate clone
    let _ = postcard::to_slice(
        &LoadApplicationResponse {
            bundle_frames: model_frames.clone(),
        },
        reply_buffer,
    )
    .map_err(serialize_failure)?;
    trace!("LOAD MODEL -> {}", model_frames);
    // Cleanup allocated slot & mark cap for release after reply completes.
    Camkes::set_reply_cap_release(model_frames.cnode);
    Ok(())
}

fn read_key_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let request =
        postcard::from_bytes::<ReadKeyRequest>(request_buffer).map_err(deserialize_failure)?;

    trace!("READ KEY bundle_id {} key {}", request.bundle_id, request.key);
    let value = unsafe { CANTRIP_SECURITY.read_key(request.bundle_id, request.key) }?;
    let _ = postcard::to_slice(&ReadKeyResponse { value }, reply_buffer).map_err(serialize_failure);
    Ok(())
}

fn write_key_request(
    request_buffer: &[u8],
    _reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let request =
        postcard::from_bytes::<WriteKeyRequest>(request_buffer).map_err(deserialize_failure)?;

    trace!(
        "WRITE KEY bundle_id {} key {} value {:?}",
        request.bundle_id,
        request.key,
        request.value
    );
    // NB: the serialized data are variable length so copy to convert
    let mut keyval = [0u8; KEY_VALUE_DATA_SIZE];
    keyval[..request.value.len()].copy_from_slice(request.value);
    unsafe { CANTRIP_SECURITY.write_key(request.bundle_id, request.key, &keyval) }
}

fn delete_key_request(
    request_buffer: &[u8],
    _reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let request =
        postcard::from_bytes::<DeleteKeyRequest>(request_buffer).map_err(deserialize_failure)?;

    trace!("DELETE KEY bundle_id {} key {}", request.bundle_id, request.key);
    unsafe { CANTRIP_SECURITY.delete_key(request.bundle_id, request.key) }
}

fn test_mailbox_request() -> Result<(), SecurityRequestError> {
    trace!("TEST MAILBOX");
    unsafe { CANTRIP_SECURITY.test_mailbox() }
}

fn capscan_request() -> Result<(), SecurityRequestError> {
    let _ = Camkes::capscan();
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn security_request(
    c_request: SecurityRequest,
    c_request_buffer_len: u32,
    c_request_buffer: *const u8,
    c_reply_buffer: *mut SecurityReplyData,
) -> SecurityRequestError {
    let request_buffer = slice::from_raw_parts(c_request_buffer, c_request_buffer_len as usize);
    let reply_buffer = &mut (*c_reply_buffer)[..];
    match c_request {
        SecurityRequest::SrEcho => echo_request(request_buffer, reply_buffer),
        SecurityRequest::SrInstall => install_request(request_buffer, reply_buffer),
        SecurityRequest::SrUninstall => uninstall_request(request_buffer, reply_buffer),
        SecurityRequest::SrSizeBuffer => size_buffer_request(request_buffer, reply_buffer),
        SecurityRequest::SrGetManifest => get_manifest_request(request_buffer, reply_buffer),
        SecurityRequest::SrLoadApplication => {
            load_application_request(request_buffer, reply_buffer)
        }
        SecurityRequest::SrLoadModel => load_model_request(request_buffer, reply_buffer),
        SecurityRequest::SrReadKey => read_key_request(request_buffer, reply_buffer),
        SecurityRequest::SrWriteKey => write_key_request(request_buffer, reply_buffer),
        SecurityRequest::SrDeleteKey => delete_key_request(request_buffer, reply_buffer),
        SecurityRequest::SrTestMailbox => test_mailbox_request(),
        SecurityRequest::SrCapScan => capscan_request(),
    }
    .map_or_else(|e| e, |_v| SecurityRequestError::SreSuccess)
}
