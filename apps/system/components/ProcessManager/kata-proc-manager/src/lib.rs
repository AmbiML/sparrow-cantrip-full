//! Cantrip OS process management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::{String, ToString};
use cantrip_proc_common as proc;
use proc::*;
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
        _pkg_buffer_size: u32,
    ) -> Result<Bundle, ProcessManagerError> {
        // Package contains: application manifest, application binary, and
        // (optional) ML workload binary to run on vector core.
        // Manifest contains bundle_id.
        // Resulting flash file/pathname is fixed (1 app / bundle), only need bundle_id.
        // Store a generated "access key" (for Tock) for start ops; this is
        // "bound via capability badging to seL4 capabilities".
        let bundle = Bundle {
            // NB: temporarily fill-in app_id
            app_id: (pkg_buffer as usize).to_string(),
            data: [0u8; 64],
        };
        Ok(bundle)
    }
    fn uninstall(&mut self, _bundle_id: &str) -> Result<(), ProcessManagerError> {
        // This is handled with the StorageManager::Installer::uninstall.
        Ok(())
    }
    fn start(&mut self, _bundle: &Bundle) -> Result<(), ProcessManagerError> {
        // 1. Allocate shared memory for the program image
        // 2. Poke security core (via mailbox) to VerifyAndLoad data and load
        //    into shared memory
        // 3. Security core responds (via mailbox) with success/failure &
        //    mailbox handler sends interrupt
        // 4. Request completed with validated program in shared memory.
        // 5. On success allocate seL4 resources: VSpace, TCB & necessary
        //    capabiltiies; setup application system context and start thread
        //    (or should resources be allocated before Verify?).
        // TODO: set up access to StorageManager? (badge seL4 cap w/ bundle_id)
        //
        // Applications with an ML workload use the MLCoordinator to request
        // data be written to the vector core.
        // TODO(sleffler): fill-in
        //        Err(ProcessManagerError::StartFailed)
        Ok(())
    }
    fn stop(&mut self, _bundle: &Bundle) -> Result<(), ProcessManagerError> {
        // 1. If thread is running, notify application so it can do cleanup;
        //    e.g. ask the MLCoordinator to stop any ML workloads
        // 2. If thread notified, wait some period of time for ack.
        // 3. If thread is running, stop thread.
        // 4. Reclaim seL4 resources: TCB, VSpace, memory, capabilities, etc.
        // TODO(sleffler): fill-in
        //        Err(ProcessManagerError::StopFailed)
        Ok(())
    }
}
