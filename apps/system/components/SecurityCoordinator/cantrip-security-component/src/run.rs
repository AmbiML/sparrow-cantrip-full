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

extern crate alloc;
use alloc::string::ToString;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use cantrip_security_coordinator::CantripSecurityCoordinator;
use cantrip_security_coordinator::CantripSecurityCoordinatorInterface;
use cantrip_security_interface::*;
use core::slice;
use log::trace;

use sel4_sys::seL4_CPtr;

static mut CAMKES: Camkes = Camkes::new("SecurityCoordinator");
static mut SECURITY_RECV_SLOT: seL4_CPtr = 0;

// cantrip_security() is unsafe to use by multiple threads. As we assume the
// caller/user is single-threaded, the function is not marked unsafe.
fn cantrip_security() -> &'static mut impl SecurityCoordinatorInterface {
    static mut CANTRIP_SECURITY: CantripSecurityCoordinator<CantripSecurityCoordinatorInterface> =
        CantripSecurityCoordinator::empty();
    unsafe {
        if CANTRIP_SECURITY.is_empty() {
            CANTRIP_SECURITY.init(CantripSecurityCoordinatorInterface::new());
        }
        CANTRIP_SECURITY.get()
    }
}

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    const HEAP_SIZE: usize = 12 * 1024;
    static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    // NB: set to max; the LoggerInterface will filter
    CAMKES.pre_init(log::LevelFilter::Trace, &mut HEAP_MEMORY);

    // Complete CANTRIP_SECURITY setup after Global allocator is setup.
    cantrip_security();

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

#[no_mangle]
pub unsafe extern "C" fn security_request(
    c_request_buffer_len: u32,
    c_request_buffer: *const u8,
    c_reply_buffer: *mut SecurityReplyData,
) -> SecurityRequestError {
    let request_buffer = slice::from_raw_parts(c_request_buffer, c_request_buffer_len as usize);
    let request = match postcard::from_bytes::<SecurityRequest>(request_buffer) {
        Ok(request) => request,
        Err(_) => return SecurityRequestError::SreDeserializeFailed,
    };

    let reply_buffer = &mut *c_reply_buffer;
    match request {
        SecurityRequest::Echo(value) => echo_request(value, reply_buffer),
        SecurityRequest::Install(pkg_contents) => {
            install_request(pkg_contents.into_owned(), reply_buffer)
        }
        SecurityRequest::InstallApp {
            app_id,
            pkg_contents,
        } => install_app_request(app_id, pkg_contents.into_owned()),
        SecurityRequest::InstallModel {
            app_id,
            model_id,
            pkg_contents,
        } => install_model_request(app_id, model_id, pkg_contents.into_owned()),
        SecurityRequest::Uninstall(bundle_id) => uninstall_request(bundle_id),
        SecurityRequest::GetPackages => get_packages_request(reply_buffer),
        SecurityRequest::SizeBuffer(bundle_id) => size_buffer_request(bundle_id, reply_buffer),
        SecurityRequest::GetManifest(bundle_id) => get_manifest_request(bundle_id, reply_buffer),
        SecurityRequest::LoadApplication(bundle_id) => {
            load_application_request(bundle_id, reply_buffer)
        }
        SecurityRequest::LoadModel {
            bundle_id,
            model_id,
        } => load_model_request(bundle_id, model_id, reply_buffer),
        SecurityRequest::ReadKey { bundle_id, key } => {
            read_key_request(bundle_id, key, reply_buffer)
        }
        SecurityRequest::WriteKey {
            bundle_id,
            key,
            value,
        } => write_key_request(bundle_id, key, value),
        SecurityRequest::DeleteKey { bundle_id, key } => delete_key_request(bundle_id, key),
        SecurityRequest::TestMailbox => test_mailbox_request(),
        SecurityRequest::CapScan => capscan_request(),
    }
    .map_or_else(|e| e, |_v| SecurityRequestError::SreSuccess)
}

fn echo_request(value: &str, reply_buffer: &mut [u8]) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    trace!("ECHO {:?}", value);
    let _ = postcard::to_slice(
        &EchoResponse {
            value: value.to_string(),
        },
        reply_buffer,
    )
    .or(Err(SecurityRequestError::SreSerializeFailed))?;
    Ok(())
}

fn install_request(
    mut pkg_contents: ObjDescBundle,
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    let recv_path = unsafe { CAMKES.get_current_recv_path() };
    Camkes::debug_assert_slot_cnode("install_request", &recv_path);

    // Move the container CNode so it's not clobbered.
    let mut container_slot = CSpaceSlot::new();
    container_slot
        .move_to(recv_path.0, recv_path.1, recv_path.2 as u8)
        .or(Err(SecurityRequestError::SreCapMoveFailed))?; // XXX expect?
    pkg_contents.cnode = container_slot.release();

    let bundle_id = cantrip_security().install(&pkg_contents)?;
    let _ = postcard::to_slice(&InstallResponse { bundle_id }, reply_buffer)
        .or(Err(SecurityRequestError::SreSerializeFailed))?;
    Ok(())
}

fn install_app_request(
    app_id: &str,
    mut pkg_contents: ObjDescBundle,
) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    let recv_path = unsafe { CAMKES.get_current_recv_path() };
    Camkes::debug_assert_slot_cnode("install_application_request", &recv_path);

    // Move the container CNode so it's not clobbered.
    let mut container_slot = CSpaceSlot::new();
    container_slot
        .move_to(recv_path.0, recv_path.1, recv_path.2 as u8)
        .or(Err(SecurityRequestError::SreCapMoveFailed))?; // XXX expect?
    pkg_contents.cnode = container_slot.release();

    cantrip_security().install_app(app_id, &pkg_contents)
}

fn install_model_request(
    app_id: &str,
    model_id: &str,
    mut pkg_contents: ObjDescBundle,
) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    let recv_path = unsafe { CAMKES.get_current_recv_path() };
    Camkes::debug_assert_slot_cnode("install_model_request", &recv_path);

    // Move the container CNode so it's not clobbered.
    let mut container_slot = CSpaceSlot::new();
    container_slot
        .move_to(recv_path.0, recv_path.1, recv_path.2 as u8)
        .or(Err(SecurityRequestError::SreCapMoveFailed))?; // XXX expect?
    pkg_contents.cnode = container_slot.release();

    cantrip_security().install_model(app_id, model_id, &pkg_contents)
}

fn uninstall_request(bundle_id: &str) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    trace!("UNINSTALL {}", bundle_id);
    cantrip_security().uninstall(bundle_id)
}

fn get_packages_request(reply_buffer: &mut [u8]) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    let bundle_ids = cantrip_security().get_packages()?;

    trace!("GET PACKAGES -> {:?}", &bundle_ids);
    // Serialize the bundle_id's in the result buffer. If we
    // overflow the buffer, an error is returned and the
    // contents are undefined (postcard does not specify).
    let _ = postcard::to_slice(&GetPackagesResponse { bundle_ids }, reply_buffer)
        .or(Err(SecurityRequestError::SreSerializeFailed))?;
    Ok(())
}

fn size_buffer_request(
    bundle_id: &str,
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();

    trace!("SIZE BUFFER bundle_id {}", bundle_id);
    let buffer_size = cantrip_security().size_buffer(bundle_id)?;
    let _ = postcard::to_slice(&SizeBufferResponse { buffer_size }, reply_buffer)
        .or(Err(SecurityRequestError::SreSerializeFailed))?;
    Ok(())
}

fn get_manifest_request(
    bundle_id: &str,
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    trace!("GET MANIFEST bundle_id {}", bundle_id);
    let manifest = cantrip_security().get_manifest(bundle_id)?;
    let _ = postcard::to_slice(&GetManifestResponse { manifest }, reply_buffer)
        .or(Err(SecurityRequestError::SreSerializeFailed))?;
    Ok(())
}

fn load_application_request(
    bundle_id: &str,
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    trace!("LOAD APPLICATION bundle_id {}", bundle_id);
    let bundle_frames = cantrip_security().load_application(bundle_id)?;
    // TODO(sleffler): maybe rearrange to eliminate clone
    let _ = postcard::to_slice(
        &LoadApplicationResponse {
            bundle_frames: bundle_frames.clone(),
        },
        reply_buffer,
    )
    .or(Err(SecurityRequestError::SreSerializeFailed))?;
    trace!("LOAD APPLICATION -> {}", bundle_frames);
    // Cleanup allocated slot & mark cap for release after reply completes.
    Camkes::set_reply_cap_release(bundle_frames.cnode);
    Ok(())
}

fn load_model_request(
    bundle_id: &str,
    model_id: &str,
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let model_frames = cantrip_security().load_model(bundle_id, model_id)?;
    // TODO(sleffler): maybe rearrange to eliminate clone
    let _ = postcard::to_slice(
        &LoadApplicationResponse {
            bundle_frames: model_frames.clone(),
        },
        reply_buffer,
    )
    .or(Err(SecurityRequestError::SreSerializeFailed))?;
    trace!("LOAD MODEL -> {}", model_frames);
    // Cleanup allocated slot & mark cap for release after reply completes.
    Camkes::set_reply_cap_release(model_frames.cnode);
    Ok(())
}

fn read_key_request(
    bundle_id: &str,
    key: &str,
    reply_buffer: &mut [u8],
) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    trace!("READ KEY bundle_id {} key {}", bundle_id, key);
    let value = cantrip_security().read_key(bundle_id, key)?;
    let _ = postcard::to_slice(&ReadKeyResponse { value: *value }, reply_buffer)
        .or(Err(SecurityRequestError::SreSerializeFailed))?;
    Ok(())
}

fn write_key_request(bundle_id: &str, key: &str, value: &[u8]) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    trace!("WRITE KEY bundle_id {} key {} value {:?}", bundle_id, key, value);
    cantrip_security().write_key(bundle_id, key, value)
}

fn delete_key_request(bundle_id: &str, key: &str) -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    trace!("DELETE KEY bundle_id {} key {}", bundle_id, key);
    cantrip_security().delete_key(bundle_id, key)
}

fn test_mailbox_request() -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    trace!("TEST MAILBOX");
    cantrip_security().test_mailbox()
}

fn capscan_request() -> Result<(), SecurityRequestError> {
    let _cleanup = Camkes::cleanup_request_cap();
    let _ = Camkes::capscan();
    Ok(())
}
