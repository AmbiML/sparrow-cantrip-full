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
// XXX for camkes.rs
#![feature(const_mut_refs)]
#![allow(dead_code)]
#![allow(unused_unsafe)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]

extern crate alloc;
use alloc::string::ToString;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::camkes;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::logger;
use cantrip_os_common::sel4_sys;
use cantrip_security_coordinator::CantripSecurityCoordinator;
use cantrip_security_coordinator::CantripSecurityCoordinatorInterface;
use cantrip_security_interface::*;
use cfg_if::cfg_if;
use log::trace;

use camkes::*;
use logger::*;

use sel4_sys::seL4_CPtr;

// Generated code...
include!(concat!(env!("SEL4_OUT_DIR"), "/../security_coordinator/camkes.rs"));

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

struct SecurityCoordinatorControlThread;
impl CamkesThreadInterface for SecurityCoordinatorControlThread {
    fn pre_init() {
        // XXX how to handle "maybe" inclusion
        static_logger!(logger);

        const HEAP_SIZE: usize = 12 * 1024;
        static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
        unsafe {
            CAMKES.pre_init(&mut HEAP_MEMORY);
        }
    }
}

cfg_if! {
    if #[cfg(feature = "CONFIG_DEBUG_BUILD")] {
        struct SecurityCoordinatorFaultHandlerThread;
        impl CamkesThreadInterface for SecurityCoordinatorFaultHandlerThread {}
    }
}

type SecurityResult = Result<Option<seL4_CPtr>, SecurityRequestError>;

struct SecurityInterfaceThread;
impl CamkesThreadInterface for SecurityInterfaceThread {
    fn run() {
        // Setup CANTRIP_SECURITY after the Global allocator is init'd.
        cantrip_security();

        let recv_slot = CSpaceSlot::new().release();
        rpc_shared_recv_with_caps!(
            security,
            recv_slot,
            SECURITY_REQUEST_DATA_SIZE,
            SecurityRequestError::Success
        );
    }
}
impl SecurityInterfaceThread {
    fn dispatch(
        _client_badge: usize,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> SecurityResult {
        let request = match postcard::from_bytes::<SecurityRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(SecurityRequestError::SreDeserializeFailed),
        };

        match request {
            SecurityRequest::Echo(value) => Self::echo_request(value, reply_buffer),
            SecurityRequest::Install(pkg_contents) => {
                Self::install_request(pkg_contents.into_owned(), reply_buffer)
            }
            SecurityRequest::InstallApp {
                app_id,
                pkg_contents,
            } => Self::install_app_request(app_id, pkg_contents.into_owned()),
            SecurityRequest::InstallModel {
                app_id,
                model_id,
                pkg_contents,
            } => Self::install_model_request(app_id, model_id, pkg_contents.into_owned()),
            SecurityRequest::Uninstall(bundle_id) => Self::uninstall_request(bundle_id),
            SecurityRequest::GetPackages => Self::get_packages_request(reply_buffer),
            SecurityRequest::SizeBuffer(bundle_id) => {
                Self::size_buffer_request(bundle_id, reply_buffer)
            }
            SecurityRequest::GetManifest(bundle_id) => {
                Self::get_manifest_request(bundle_id, reply_buffer)
            }
            SecurityRequest::LoadApplication(bundle_id) => {
                Self::load_application_request(bundle_id, reply_buffer)
            }
            SecurityRequest::LoadModel {
                bundle_id,
                model_id,
            } => Self::load_model_request(bundle_id, model_id, reply_buffer),
            SecurityRequest::ReadKey { bundle_id, key } => {
                Self::read_key_request(bundle_id, key, reply_buffer)
            }
            SecurityRequest::WriteKey {
                bundle_id,
                key,
                value,
            } => Self::write_key_request(bundle_id, key, value),
            SecurityRequest::DeleteKey { bundle_id, key } => {
                Self::delete_key_request(bundle_id, key)
            }
            SecurityRequest::TestMailbox => Self::test_mailbox_request(),
            SecurityRequest::CapScan => Self::capscan_request(),
        }
    }
    fn echo_request(value: &str, reply_buffer: &mut [u8]) -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        trace!("ECHO {:?}", value);
        let _ = postcard::to_slice(
            &EchoResponse {
                value: value.to_string(),
            },
            reply_buffer,
        )
        .or(Err(SecurityRequestError::SreSerializeFailed))?;
        Ok(None)
    }
    fn install_request(mut pkg_contents: ObjDescBundle, reply_buffer: &mut [u8]) -> SecurityResult {
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
        Ok(None)
    }
    fn install_app_request(app_id: &str, mut pkg_contents: ObjDescBundle) -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        let recv_path = unsafe { CAMKES.get_current_recv_path() };
        Camkes::debug_assert_slot_cnode("install_application_request", &recv_path);

        // Move the container CNode so it's not clobbered.
        let mut container_slot = CSpaceSlot::new();
        container_slot
            .move_to(recv_path.0, recv_path.1, recv_path.2 as u8)
            .or(Err(SecurityRequestError::SreCapMoveFailed))?; // XXX expect?
        pkg_contents.cnode = container_slot.release();

        cantrip_security()
            .install_app(app_id, &pkg_contents)
            .map(|_| None)
    }
    fn install_model_request(
        app_id: &str,
        model_id: &str,
        mut pkg_contents: ObjDescBundle,
    ) -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        let recv_path = unsafe { CAMKES.get_current_recv_path() };
        Camkes::debug_assert_slot_cnode("install_model_request", &recv_path);

        // Move the container CNode so it's not clobbered.
        let mut container_slot = CSpaceSlot::new();
        container_slot
            .move_to(recv_path.0, recv_path.1, recv_path.2 as u8)
            .or(Err(SecurityRequestError::SreCapMoveFailed))?; // XXX expect?
        pkg_contents.cnode = container_slot.release();

        cantrip_security()
            .install_model(app_id, model_id, &pkg_contents)
            .map(|_| None)
    }
    fn uninstall_request(bundle_id: &str) -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        trace!("UNINSTALL {}", bundle_id);
        cantrip_security().uninstall(bundle_id).map(|_| None)
    }
    fn get_packages_request(reply_buffer: &mut [u8]) -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        let bundle_ids = cantrip_security().get_packages()?;

        trace!("GET PACKAGES -> {:?}", &bundle_ids);
        // Serialize the bundle_id's in the result buffer. If we
        // overflow the buffer, an error is returned and the
        // contents are undefined (postcard does not specify).
        let _ = postcard::to_slice(&GetPackagesResponse { bundle_ids }, reply_buffer)
            .or(Err(SecurityRequestError::SreSerializeFailed))?;
        Ok(None)
    }
    fn size_buffer_request(bundle_id: &str, reply_buffer: &mut [u8]) -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        trace!("SIZE BUFFER bundle_id {}", bundle_id);
        let buffer_size = cantrip_security().size_buffer(bundle_id)?;
        let _ = postcard::to_slice(&SizeBufferResponse { buffer_size }, reply_buffer)
            .or(Err(SecurityRequestError::SreSerializeFailed))?;
        Ok(None)
    }
    fn get_manifest_request(bundle_id: &str, reply_buffer: &mut [u8]) -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        trace!("GET MANIFEST bundle_id {}", bundle_id);
        let manifest = cantrip_security().get_manifest(bundle_id)?;
        let _ = postcard::to_slice(&GetManifestResponse { manifest }, reply_buffer)
            .or(Err(SecurityRequestError::SreSerializeFailed))?;
        Ok(None)
    }
    fn load_application_request(bundle_id: &str, reply_buffer: &mut [u8]) -> SecurityResult {
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
        Ok(Some(bundle_frames.cnode))
    }
    fn load_model_request(
        bundle_id: &str,
        model_id: &str,
        reply_buffer: &mut [u8],
    ) -> SecurityResult {
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
        Ok(Some(model_frames.cnode))
    }
    fn read_key_request(bundle_id: &str, key: &str, reply_buffer: &mut [u8]) -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        trace!("READ KEY bundle_id {} key {}", bundle_id, key);
        let value = cantrip_security().read_key(bundle_id, key)?;
        let _ = postcard::to_slice(&ReadKeyResponse { value: *value }, reply_buffer)
            .or(Err(SecurityRequestError::SreSerializeFailed))?;
        Ok(None)
    }
    fn write_key_request(bundle_id: &str, key: &str, value: &[u8]) -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        trace!("WRITE KEY bundle_id {} key {} value {:?}", bundle_id, key, value);
        cantrip_security()
            .write_key(bundle_id, key, value)
            .map(|_| None)
    }
    fn delete_key_request(bundle_id: &str, key: &str) -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        trace!("DELETE KEY bundle_id {} key {}", bundle_id, key);
        cantrip_security().delete_key(bundle_id, key).map(|_| None)
    }
    fn test_mailbox_request() -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        trace!("TEST MAILBOX");
        cantrip_security().test_mailbox().map(|_| None)
    }
    fn capscan_request() -> SecurityResult {
        let _cleanup = Camkes::cleanup_request_cap();
        let _ = Camkes::capscan();
        Ok(None)
    }
}
