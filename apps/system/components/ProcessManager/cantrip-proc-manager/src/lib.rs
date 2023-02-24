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

//! Cantrip OS process management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::String;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_proc_interface::Bundle;
use cantrip_proc_interface::BundleIdArray;
use cantrip_proc_interface::BundleImplInterface;
use cantrip_proc_interface::PackageManagementInterface;
use cantrip_proc_interface::ProcessControlInterface;
use cantrip_proc_interface::ProcessManagerError;
use cantrip_proc_interface::ProcessManagerInterface;
use cantrip_security_interface::cantrip_security_install;
use cantrip_security_interface::cantrip_security_install_application;
use cantrip_security_interface::cantrip_security_load_application;
use cantrip_security_interface::cantrip_security_uninstall;
use log::trace;
use spin::Mutex;
use spin::MutexGuard;

mod sel4bundle;
use sel4bundle::seL4BundleImpl;

mod proc_manager;
pub use proc_manager::ProcessManager;

// CantripProcManager bundles an instance of the ProcessManager that operates
// on CantripOS interfaces and synchronizes public use with a Mutex. There is
// a two-step dance to setup an instance because we want CANTRIP_PROC static
// and ProcessManager is incapable of supplying a const fn due it's use of
// hashbrown::HashMap.
pub struct CantripProcManager {
    manager: Mutex<Option<ProcessManager<CantripManagerInterface>>>,
}
impl CantripProcManager {
    // Constructs a partially-initialized instance; to complete call init().
    // This is needed because we need a const fn for static setup and with
    // that constraint we cannot reference self.interface.
    pub const fn empty() -> Self {
        Self {
            manager: Mutex::new(None),
        }
    }

    pub fn get(&self) -> Guard {
        Guard {
            manager: self.manager.lock(),
        }
    }
}
pub struct Guard<'a> {
    manager: MutexGuard<'a, Option<ProcessManager<CantripManagerInterface>>>,
}
impl Guard<'_> {
    pub fn is_empty(&self) -> bool { self.manager.is_none() }
    // Finishes the setup started by empty():
    pub fn init(&mut self) {
        assert!(self.manager.is_none());
        *self.manager = Some(ProcessManager::new(CantripManagerInterface));
    }
    // Returns the bundle capacity.
    pub fn capacity(&self) -> usize { self.manager.as_ref().unwrap().capacity() }
}
// These just lock accesses and handle the necessary indirection.
impl PackageManagementInterface for Guard<'_> {
    fn install(&mut self, pkg_contents: &ObjDescBundle) -> Result<String, ProcessManagerError> {
        self.manager.as_mut().unwrap().install(pkg_contents)
    }
    fn install_app(
        &mut self,
        app_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), ProcessManagerError> {
        self.manager
            .as_mut()
            .unwrap()
            .install_app(app_id, pkg_contents)
    }
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        self.manager.as_mut().unwrap().uninstall(bundle_id)
    }
}
impl ProcessControlInterface for Guard<'_> {
    fn start(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        self.manager.as_mut().unwrap().start(bundle_id)
    }
    fn stop(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        self.manager.as_mut().unwrap().stop(bundle_id)
    }
    fn get_running_bundles(&self) -> Result<BundleIdArray, ProcessManagerError> {
        self.manager.as_ref().unwrap().get_running_bundles()
    }
    fn capscan(&self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        self.manager.as_ref().unwrap().capscan(bundle_id)
    }
}

struct CantripManagerInterface;
impl ProcessManagerInterface for CantripManagerInterface {
    type BundleImpl = seL4BundleImpl;

    fn install(&mut self, pkg_contents: &ObjDescBundle) -> Result<String, ProcessManagerError> {
        trace!("ProcessManagerInterface::install pkg_contents {}", pkg_contents);

        // Package contains: application manifest, application binary, and
        // (optional) ML workload binary to run on vector core.
        // Manifest contains bundle_id.
        // Resulting flash file/pathname is fixed (1 app / bundle), only need bundle_id.
        // Pass opaque package contents through; get back bundle_id.

        // This is handled by the SecurityCoordinator.
        Ok(cantrip_security_install(pkg_contents)?)
    }
    fn install_app(
        &mut self,
        app_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), ProcessManagerError> {
        trace!(
            "ProcessManagerInterface::install_app {} pkg_contents {}",
            app_id,
            pkg_contents
        );
        Ok(cantrip_security_install_application(app_id, pkg_contents)?)
    }
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        trace!("ProcessManagerInterface::uninstall bundle_id {}", bundle_id);

        // NB: the caller has already checked no running application exists
        // NB: the Security Core is assumed to invalidate/remove any kv store

        // This is handled by the SecurityCoordinator.
        Ok(cantrip_security_uninstall(bundle_id)?)
    }
    fn start(&mut self, bundle: &Bundle) -> Result<Self::BundleImpl, ProcessManagerError> {
        trace!("ProcessManagerInterface::start {:?}", bundle);

        // Design doc says:
        // 1. Ask security core for application footprint with SizeBuffer
        // 2. Ask security core for manifest (maybe piggyback on SizeBuffer)
        //    and parse for necessary info (e.g. whether kv Storage is
        //    required, other privileges/capabilities)
        // 3. Ask MemoryManager for shared memory pages for the application
        //    (model handled separately by MlCoordinator since we do not know
        //    which model will be used)
        // 4. Allocate other seL4 resources:
        //     - VSpace, TCB & necessary capabiltiies
        // 5. Ask security core to VerifyAndLoad app into shared memory pages
        // 6. Complete seL4 setup:
        //     - Setup application system context and start thread
        //     - Badge seL4 recv cap w/ bundle_id for (optional) StorageManager
        //       access
        // What we do atm is:
        // 1. Ask SecurityCoordinator to return the application contents to load.
        //    Data are delivered as a read-only ObjDescBundle ready to copy into
        //    the VSpace.
        // 2. Do 4+6 with BundleImplInterface::start.

        // TODO(sleffler): awkward container_slot ownership
        let mut container_slot = CSpaceSlot::new();
        let bundle_frames = cantrip_security_load_application(&bundle.app_id, &container_slot)?;
        let mut sel4_bundle = seL4BundleImpl::new(bundle, &bundle_frames)?;
        // sel4_bundle owns container_slot now; release our ref so it's not
        // reclaimed when container_slot goes out of scope.
        container_slot.release();

        sel4_bundle.start()?;

        Ok(sel4_bundle)
    }
    fn stop(&mut self, bundle_impl: &mut Self::BundleImpl) -> Result<(), ProcessManagerError> {
        trace!("ProcessManagerInterface::stop");

        // 0. Assume thread is running (caller verifies)
        // 1. Notify application so it can do cleanup; e.g. ask the
        //    MLCoordinator to stop any ML workloads
        // 2. Wait some period of time for an ack from application
        // 3. Stop thread
        // 4. Reclaim seL4 resources: TCB, VSpace, memory, capabilities, etc.
        // TODO(sleffler): fill-in 1+2
        bundle_impl.stop()
    }
    fn capscan(&self, bundle_impl: &Self::BundleImpl) -> Result<(), ProcessManagerError> {
        trace!("ProcessManagerInterface::capscan");

        bundle_impl.capscan()
    }
}
