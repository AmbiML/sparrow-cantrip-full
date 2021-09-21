//! Cantrip OS process management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::String;
use cantrip_proc_common::*;
use cantrip_security_interface::cantrip_security_request;
use cantrip_security_interface::InstallRequest;
use cantrip_security_interface::SecurityRequest;
use cantrip_security_interface::UninstallRequest;
use cantrip_security_interface::SECURITY_REPLY_DATA_SIZE;
use log::trace;
use postcard;
use spin::Mutex;

mod proc_manager;
pub use proc_manager::ProcessManager;

// NB: CANTRIP_PROC cannot be used before setup is completed with a call to init()
#[cfg(not(test))]
pub static mut CANTRIP_PROC: CantripProcManager = CantripProcManager::empty();

// CantripProcManager bundles an instance of the ProcessManager that operates
// on CantripOS interfaces and synchronizes public use with a Mutex. There is
// a two-step dance to setup an instance because we want CANTRIP_PROC static
// and ProcessManager is incapable of supplying a const fn due it's use of
// hashbrown::HashMap.
pub struct CantripProcManager {
    manager: Mutex<Option<ProcessManager>>,
}
impl CantripProcManager {
    // Constructs a partially-initialized instance; to complete call init().
    // This is needed because we need a const fn for static setup and with
    // that constraint we cannot reference self.interface.
    const fn empty() -> CantripProcManager {
        CantripProcManager {
            manager: Mutex::new(None),
        }
    }

    // Finishes the setup started by empty():
    pub fn init(&self) {
        *self.manager.lock() = Some(ProcessManager::new(CantripManagerInterface));
    }

    // Returns the bundle capacity.
    pub fn capacity(&self) -> usize {
        self.manager.lock().as_ref().unwrap().capacity()
    }
}
// These just lock accesses and handle the necessary indirection.
impl PackageManagementInterface for CantripProcManager {
    fn install(
        &mut self,
        pkg_buffer: *const u8,
        pkg_buffer_len: usize,
    ) -> Result<String, ProcessManagerError> {
        self.manager
            .lock()
            .as_mut()
            .unwrap()
            .install(pkg_buffer, pkg_buffer_len)
    }
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        self.manager.lock().as_mut().unwrap().uninstall(bundle_id)
    }
}
impl ProcessControlInterface for CantripProcManager {
    fn start(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        self.manager.lock().as_mut().unwrap().start(bundle_id)
    }
    fn stop(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        self.manager.lock().as_mut().unwrap().stop(bundle_id)
    }
    fn get_running_bundles(&self) -> Result<BundleIdArray, ProcessManagerError> {
        self.manager.lock().as_ref().unwrap().get_running_bundles()
    }
}

struct CantripManagerInterface;
impl ProcessManagerInterface for CantripManagerInterface {
    fn install(
        &mut self,
        pkg_buffer: *const u8,
        pkg_buffer_size: u32,
    ) -> Result<String, ProcessManagerError> {
        // Package contains: application manifest, application binary, and
        // (optional) ML workload binary to run on vector core.
        // Manifest contains bundle_id.
        // Resulting flash file/pathname is fixed (1 app / bundle), only need bundle_id.
        // Pass opaque package contents through; get back bundle_id.

        // This is handled by the SecurityCoordinator.
        let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
        match cantrip_security_request(
            SecurityRequest::SrInstall,
            &InstallRequest {
                pkg_buffer_size: pkg_buffer_size,
                pkg_buffer: pkg_buffer,
            },
            reply,
        ) {
            Ok(_) => {
                fn deserialize_failure(e: postcard::Error) -> ProcessManagerError {
                    trace!("install failed: deserialize {:?}", e);
                    ProcessManagerError::BundleDataInvalid
                }
                postcard::from_bytes::<String>(reply).map_err(deserialize_failure)
            }
            Err(status) => {
                trace!("install failed: {:?}", status);
                Err(ProcessManagerError::InstallFailed)
            }
        }
    }
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        // NB: the caller has already checked no running application exists
        // NB: the Security Core is assumed to invalidate/remove any kv store

        // This is handled by the SecurityCoordinator.
        let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
        match cantrip_security_request(
            SecurityRequest::SrUninstall,
            &UninstallRequest {
                bundle_id: &bundle_id,
            },
            reply,
        ) {
            Ok(_) => Ok(()),
            Err(status) => {
                trace!("uninstall failed: {:?}", status);
                Err(ProcessManagerError::UninstallFailed)
            }
        }
    }
    fn start(&mut self, _bundle: &Bundle) -> Result<(), ProcessManagerError> {
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
        //
        // Applications with an ML workload use the MlCoordinator to request
        // data be loaded for the vector core.
        //
        // TBD where stuff normally in ELF headers comes from (e.g. starting pc,
        // text size for marking pages executable, bss size).
        //
        // May want stack size parameterized.
        //
        // TODO(sleffler): fill-in
        //        Err(ProcessManagerError::StartFailed)
        Ok(())
    }
    fn stop(&mut self, _bundle: &Bundle) -> Result<(), ProcessManagerError> {
        // 0. Assume thread is running (caller verifies)
        // 1. Notify application so it can do cleanup; e.g. ask the
        //    MLCoordinator to stop any ML workloads
        // 2. Wait some period of time for an ack from application
        // 3. Stop thread
        // 4. Reclaim seL4 resources: TCB, VSpace, memory, capabilities, etc.
        // TODO(sleffler): fill-in
        //        Err(ProcessManagerError::StopFailed)
        Ok(())
    }
}
