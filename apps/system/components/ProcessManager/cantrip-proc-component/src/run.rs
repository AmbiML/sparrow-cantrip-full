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

//! Cantrip OS ProcessManager component support.

// Code here binds the camkes component to the rust code.
#![no_std]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use cantrip_proc_interface::*;
use cantrip_proc_manager::CantripProcManager;
use core::slice;
use log::trace;

use sel4_sys::seL4_CPtr;

static mut CAMKES: Camkes = Camkes::new("ProcessManager");
// NB: CANTRIP_PROC cannot be used before setup is completed with a call to init()
static mut CANTRIP_PROC: CantripProcManager = CantripProcManager::empty();

// TODO(sleffler): 0 is valid
static mut PKG_MGMT_RECV_SLOT: seL4_CPtr = 0;

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static mut HEAP_MEMORY: [u8; 16 * 1024] = [0; 16 * 1024];
    CAMKES.pre_init(log::LevelFilter::Trace, &mut HEAP_MEMORY);

    // Complete CANTRIP_PROC setup now that Global allocator is setup.
    CANTRIP_PROC.init();
    trace!("ProcessManager has capacity for {} bundles", CANTRIP_PROC.capacity());

    PKG_MGMT_RECV_SLOT = CSpaceSlot::new().release();
}

#[no_mangle]
pub unsafe extern "C" fn pkg_mgmt__init() {
    // Point the receive path to the well-known slot for receiving
    // CNode's from clients for pkg_mgmt requests.
    //
    // NB: this must be done here (rather than someplace like pre_init)
    // so it's in the context of the PackageManagementInterface thread
    // (so we write the correct ipc buffer).
    CAMKES.init_recv_path(&Camkes::top_level_path(PKG_MGMT_RECV_SLOT));
}

#[no_mangle]
pub unsafe extern "C" fn pkg_mgmt_request(
    c_request_buffer_len: u32,
    c_request_buffer: *const u8,
    c_reply_buffer: *mut RawBundleIdData,
) -> ProcessManagerError {
    let _cleanup = Camkes::cleanup_request_cap();
    let request_buffer = slice::from_raw_parts(c_request_buffer, c_request_buffer_len as usize);
    let request = match postcard::from_bytes::<PackageManagementRequest>(request_buffer) {
        Ok(request) => request,
        Err(_) => return ProcessManagerError::DeserializeError,
    };

    match request {
        PackageManagementRequest::Install(pkg_contents) => {
            install_request(pkg_contents.into_owned(), &mut *c_reply_buffer)
        }
        PackageManagementRequest::InstallApp {
            app_id,
            pkg_contents,
        } => install_app_request(app_id, pkg_contents.into_owned(), &mut *c_reply_buffer),
        PackageManagementRequest::Uninstall(bundle_id) => {
            uninstall_request(bundle_id, &mut *c_reply_buffer)
        }
    }
    .map_or_else(|e| e, |()| ProcessManagerError::Success)
}

fn install_request(
    mut pkg_contents: ObjDescBundle,
    reply_buffer: &mut RawBundleIdData,
) -> Result<(), ProcessManagerError> {
    // NB: make sure noone clobbers the setup done in pkg_mgmt__init,
    // and clear any capability the path points to when dropped
    let recv_path = unsafe { CAMKES.get_owned_current_recv_path() };
    Camkes::debug_assert_slot_cnode("install_request", &recv_path);

    pkg_contents.cnode = recv_path.1;

    let bundle_id = unsafe { CANTRIP_PROC.install(&pkg_contents) }?;
    let _ = postcard::to_slice(&InstallResponse { bundle_id }, reply_buffer)
        .or(Err(ProcessManagerError::SerializeError))?;
    Ok(())
}

fn install_app_request(
    app_id: &str,
    mut pkg_contents: ObjDescBundle,
    _reply_buffer: &mut RawBundleIdData,
) -> Result<(), ProcessManagerError> {
    // NB: make sure noone clobbers the setup done in pkg_mgmt__init,
    // and clear any capability the path points to when dropped
    let recv_path = unsafe { CAMKES.get_owned_current_recv_path() };
    Camkes::debug_assert_slot_cnode("install_app_request", &recv_path);

    pkg_contents.cnode = recv_path.1;

    unsafe { CANTRIP_PROC.install_app(app_id, &pkg_contents) }
}

fn uninstall_request(
    bundle_id: &str,
    _reply_buffer: &mut RawBundleIdData,
) -> Result<(), ProcessManagerError> {
    // NB: make sure noone clobbers the setup done in pkg_mgmt__init,
    // and clear any capability the path points to when dropped
    let recv_path = unsafe { CAMKES.get_owned_current_recv_path() };
    Camkes::debug_assert_slot_empty("uninstall_request", &recv_path);

    let _ = unsafe { CANTRIP_PROC.uninstall(bundle_id) }?;
    Camkes::debug_assert_slot_empty("uninstall_request", &recv_path);
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn proc_ctrl_request(
    c_request_buffer_len: u32,
    c_request_buffer: *const u8,
    c_reply_buffer: *mut RawBundleIdData,
) -> ProcessManagerError {
    let _cleanup = Camkes::cleanup_request_cap();
    let request_buffer = slice::from_raw_parts(c_request_buffer, c_request_buffer_len as usize);
    let request = match postcard::from_bytes::<ProcessControlRequest>(request_buffer) {
        Ok(request) => request,
        Err(_) => return ProcessManagerError::DeserializeError,
    };

    match request {
        ProcessControlRequest::Start(bundle_id) => start_request(bundle_id, &mut *c_reply_buffer),
        ProcessControlRequest::Stop(bundle_id) => stop_request(bundle_id, &mut *c_reply_buffer),
        ProcessControlRequest::GetRunningBundles => {
            get_running_bundles_request(&mut *c_reply_buffer)
        }

        ProcessControlRequest::CapScan => capscan_request(),
        ProcessControlRequest::CapScanBundle(bundle_id) => {
            capscan_bundle_request(bundle_id, &mut *c_reply_buffer)
        }
    }
    .map_or_else(|e| e, |()| ProcessManagerError::Success)
}

fn start_request(
    bundle_id: &str,
    _reply_buffer: &mut RawBundleIdData,
) -> Result<(), ProcessManagerError> {
    unsafe { CANTRIP_PROC.start(bundle_id) }
}

fn stop_request(bundle_id: &str, _reply_buffer: &mut [u8]) -> Result<(), ProcessManagerError> {
    unsafe { CANTRIP_PROC.stop(bundle_id) }
}

fn get_running_bundles_request(
    reply_buffer: &mut RawBundleIdData,
) -> Result<(), ProcessManagerError> {
    let bundle_ids = unsafe { CANTRIP_PROC.get_running_bundles() }?;
    // Serialize the bundle_id's in the result buffer. If we
    // overflow the buffer, an error is returned and the
    // contents are undefined (postcard does not specify).
    let _ = postcard::to_slice(&GetRunningBundlesResponse { bundle_ids }, reply_buffer)
        .or(Err(ProcessManagerError::DeserializeError))?;
    Ok(())
}

fn capscan_request() -> Result<(), ProcessManagerError> {
    let _ = Camkes::capscan();
    Ok(())
}

fn capscan_bundle_request(
    bundle_id: &str,
    _reply_buffer: &mut [u8],
) -> Result<(), ProcessManagerError> {
    unsafe { CANTRIP_PROC.capscan(bundle_id) }
}
