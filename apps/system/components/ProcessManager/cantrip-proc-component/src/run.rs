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
// XXX for camkes.rs
#![feature(const_mut_refs)]
#![allow(dead_code)]
#![allow(unused_unsafe)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]

extern crate alloc;
use alloc::string::String;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::camkes;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::logger;
use cantrip_os_common::sel4_sys;
use cantrip_proc_interface::*;
use cantrip_proc_manager::CantripProcManager;
use cfg_if::cfg_if;
use log::trace;

use camkes::*;
use logger::*;

use sel4_sys::seL4_CPtr;

// Generated code...
include!(concat!(env!("SEL4_OUT_DIR"), "/../process_manager/camkes.rs"));

fn cantrip_proc() -> impl PackageManagementInterface + ProcessControlInterface {
    static CANTRIP_PROC: CantripProcManager = CantripProcManager::empty();
    let mut manager = CANTRIP_PROC.get();
    if manager.is_empty() {
        // Complete CANTRIP_PROC setup now that Global allocator is setup.
        manager.init();
        trace!("ProcessManager has capacity for {} bundles", manager.capacity());
    }
    manager
}

struct ProcessManagerControlThread;
impl CamkesThreadInterface for ProcessManagerControlThread {
    fn pre_init() {
        // XXX how to handle "maybe" inclusion
        static_logger!(logger);

        static mut HEAP_MEMORY: [u8; 16 * 1024] = [0; 16 * 1024];
        unsafe {
            CAMKES.pre_init(&mut HEAP_MEMORY);
        }
    }
}

cfg_if! {
    if #[cfg(feature = "CONFIG_DEBUG_BUILD")] {
        struct ProcessManagerFaultHandlerThread;
        impl CamkesThreadInterface for ProcessManagerFaultHandlerThread {}
    }
}

// TODO(sleffler): 0 is valid
static mut PKG_MGMT_RECV_SLOT: seL4_CPtr = 0;

type PkgMgmtResult = Result<Option<seL4_CPtr>, ProcessManagerError>;

struct PkgMgmtInterfaceThread;
impl CamkesThreadInterface for PkgMgmtInterfaceThread {
    fn init() {
        unsafe {
            PKG_MGMT_RECV_SLOT = CSpaceSlot::new().release();
        }
    }
    fn run() {
        rpc_shared_recv_with_caps!(
            pkg_mgmt,
            PKG_MGMT_RECV_SLOT,
            PKG_MGMT_REQUEST_DATA_SIZE,
            ProcessManagerError::Success
        );
    }
}
impl PkgMgmtInterfaceThread {
    fn dispatch(
        _client_badge: usize,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> PkgMgmtResult {
        let _cleanup = Camkes::cleanup_request_cap();
        let request = match postcard::from_bytes::<PackageManagementRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(ProcessManagerError::DeserializeError),
        };

        match request {
            PackageManagementRequest::Install(pkg_contents) => {
                Self::install_request(pkg_contents.into_owned(), reply_buffer)
            }
            PackageManagementRequest::InstallApp {
                app_id,
                pkg_contents,
            } => Self::install_app_request(app_id, pkg_contents.into_owned()),
            PackageManagementRequest::Uninstall(bundle_id) => Self::uninstall_request(bundle_id),
        }
    }
    fn install_request(mut pkg_contents: ObjDescBundle, reply_buffer: &mut [u8]) -> PkgMgmtResult {
        // NB: make sure noone clobbers the setup done in init(),
        // and clear any capability the path points to when dropped
        let recv_path = unsafe { CAMKES.get_owned_current_recv_path() };
        Camkes::debug_assert_slot_cnode("install_request", &recv_path);

        pkg_contents.cnode = recv_path.1;

        let bundle_id = cantrip_proc().install(&pkg_contents)?;
        let _ = postcard::to_slice(&InstallResponse { bundle_id }, reply_buffer)
            .or(Err(ProcessManagerError::SerializeError))?;
        Ok(None)
    }
    fn install_app_request(app_id: &str, mut pkg_contents: ObjDescBundle) -> PkgMgmtResult {
        // NB: make sure noone clobbers the setup done in init(),
        // and clear any capability the path points to when dropped
        let recv_path = unsafe { CAMKES.get_owned_current_recv_path() };
        Camkes::debug_assert_slot_cnode("install_app_request", &recv_path);

        pkg_contents.cnode = recv_path.1;

        cantrip_proc()
            .install_app(app_id, &pkg_contents)
            .map(|_| None)
    }
    fn uninstall_request(bundle_id: &str) -> PkgMgmtResult {
        // NB: make sure noone clobbers the setup done in pkg_mgmt__init,
        // and clear any capability the path points to when dropped
        let recv_path = unsafe { CAMKES.get_owned_current_recv_path() };
        Camkes::debug_assert_slot_empty("uninstall_request", &recv_path);

        cantrip_proc().uninstall(bundle_id)?;
        Camkes::debug_assert_slot_empty("uninstall_request", &recv_path);
        Ok(None)
    }
}

type ProcCtrlResult = Result<usize, ProcessManagerError>;

struct ProcCtrlInterfaceThread;
impl CamkesThreadInterface for ProcCtrlInterfaceThread {
    fn run() {
        rpc_basic_recv!(proc_ctrl, PROC_CTRL_REQUEST_DATA_SIZE, ProcessManagerError::Success);
    }
}
impl ProcCtrlInterfaceThread {
    fn dispatch(
        _client_badge: usize,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> ProcCtrlResult {
        // XXX needed still?
        let _cleanup = Camkes::cleanup_request_cap();
        let request = match postcard::from_bytes::<ProcessControlRequest>(request_buffer) {
            Ok(request) => request,
            Err(_) => return Err(ProcessManagerError::DeserializeError),
        };

        match request {
            ProcessControlRequest::Start(bundle_id) => Self::start_request(bundle_id),
            ProcessControlRequest::Stop(bundle_id) => Self::stop_request(bundle_id),
            ProcessControlRequest::GetRunningBundles => {
                Self::get_running_bundles_request(reply_buffer)
            }

            ProcessControlRequest::CapScan => Self::capscan_request(),
            ProcessControlRequest::CapScanBundle(bundle_id) => {
                Self::capscan_bundle_request(bundle_id)
            }
        }
    }
    fn start_request(bundle_id: &str) -> ProcCtrlResult {
        // TODO(283265795): copy bundle_id from the IPCBuffer
        cantrip_proc().start(&String::from(bundle_id)).map(|_| 0)
    }
    fn stop_request(bundle_id: &str) -> ProcCtrlResult {
        // TODO(283265795): copy bundle_id from the IPCBuffer
        cantrip_proc().stop(&String::from(bundle_id)).map(|_| 0)
    }
    fn get_running_bundles_request(reply_buffer: &mut [u8]) -> ProcCtrlResult {
        let bundle_ids = cantrip_proc().get_running_bundles()?;
        // Serialize the bundle_id's in the result buffer. If we
        // overflow the buffer, an error is returned and the
        // contents are undefined (postcard does not specify).
        let reply_slice =
            postcard::to_slice(&GetRunningBundlesResponse { bundle_ids }, reply_buffer)
                .or(Err(ProcessManagerError::DeserializeError))?;
        Ok(reply_slice.len())
    }
    fn capscan_request() -> ProcCtrlResult {
        let _ = Camkes::capscan();
        Ok(0)
    }
    fn capscan_bundle_request(bundle_id: &str) -> ProcCtrlResult {
        // TODO(283265795): copy bundle_id from the IPCBuffer
        cantrip_proc().capscan(&String::from(bundle_id)).map(|_| 0)
    }
}
