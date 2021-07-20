//! Cantrip OS process management support

#![cfg_attr(not(test), no_std)]

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
// a two-step dance to setup an instance because we embed CantripManagerInterface
// in the same struct and because we want CANTRIP_PROC static and ProcessManager
// is incapable of supplying a const fn due it's use of hashbrown::HashMap.
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
    fn install(&mut self, bundle_id: &str, bundle: &Bundle) -> Result<(), ProcessManagerError> {
        self.manager
            .lock()
            .as_mut()
            .unwrap()
            .install(bundle_id, bundle)
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
    fn install(&self, _bundle: &Bundle) -> Result<(), ProcessManagerError> {
        // Package contains: application manifest, application binary, and
        // (optional) ML workload binary to run on vector core.
        // Manifest contains bundle_id.
        // Generated flash file/pathname is not useful, always use bundle_id.
        // Store a generated "access key" (for Tock) for start ops; this is
        // "bound via capability badging to seL4 capabilities".
        Ok(())
    }
    fn uninstall(&self, _bundle: &Bundle) -> Result<(), ProcessManagerError> {
        // Only need bundle_id; shouldn't we need a key too?
        Ok(())
    }
    fn start(&self, _bundle: &Bundle) -> Result<(), ProcessManagerError> {
        // 1. Allocate shared memory for the program image
        // 2. Poke security core (via mailbox) to VerifyAndLoad data and load
        //    into shared memory
        // 3. Security core responds (via mailbox) with success/failure &
        //    mailbox handler sends interrupt
        // 4. Request completed with validated program in shared memory.
        // 5. On success allocate seL4 resources: VSpace, TCB & necessary
        //    capabiltiies; setup application system context and start thread
        //    (or should resources be allocated before Verify?).
        //
        // Applications with an ML workload use the MLCoordinator to request
        // data be written to the vector core.
        Ok(())
    }
    fn stop(&self, _bundle: &Bundle) -> Result<(), ProcessManagerError> {
        // 1. If thread is running, notify application so it can do cleanup;
        //    e.g. ask the MLCoordinator to stop any ML workloads
        // 2. If thread notified, wait some period of time for ack.
        // 3. If thread is running, stop thread.
        // 4. Reclaim seL4 resources: TCB, VSpace, memory, capabilities, etc.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc::ProcessManagerError as pme;

    #[test]
    fn test_pkg_mgmt() {
        let mut mgr = CantripProcManager::empty();
        mgr.init();

        let bundle_id = "foo";
        let bundle = Bundle::new();

        // Not installed, should fail.
        assert_eq!(mgr.uninstall(bundle_id).err(), Some(pme::BundleNotFound));
        // Install the bundle.
        assert!(mgr.install(bundle_id, &bundle).is_ok());
        // Re-install the same bundle should fail.
        assert_eq!(
            mgr.install(bundle_id, &bundle).err(),
            Some(pme::BundleFound)
        );
        // Now uninstalling the bundle should work.
        assert!(mgr.uninstall(bundle_id).is_ok());
    }

    #[test]
    fn test_proc_ctrl() {
        let mut mgr = CantripProcManager::empty();
        mgr.init();

        let bid2 = "2";
        let bundle2 = Bundle::new();
        let bid9 = "9";
        let bundle9 = Bundle::new();

        assert!(mgr.install(bid2, &bundle2).is_ok());
        assert!(mgr.install(bid9, &bundle9).is_ok());
        assert!(mgr.stop(bid2).is_ok());
        assert!(mgr.start(bid2).is_ok());
        assert!(mgr.start(bid9).is_ok());

        let running = mgr.get_running_bundles().unwrap();
        assert_eq!(running.len(), 2);
        assert!(running.find(bid2));
        assert!(running.find(bid9));

        assert!(mgr.stop(bid2).is_ok());
        // After stopping the bundle we should see nothing running.
        let running = mgr.get_running_bundles().unwrap();
        assert_eq!(running.len(), 1);
        assert!(running.find(bid9));

        assert!(mgr.stop(bid9).is_ok());
        // After stopping the bundle we should see nothing running.
        let running = mgr.get_running_bundles().unwrap();
        assert_eq!(running.len(), 0);
    }
}
