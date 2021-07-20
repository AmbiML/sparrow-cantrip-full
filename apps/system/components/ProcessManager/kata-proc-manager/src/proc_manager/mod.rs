//! Cantrip OS process management support

extern crate alloc;
use alloc::boxed::Box;
use core::marker::Sync;
use hashbrown::HashMap;
use log::trace;

use cantrip_proc_common as proc;
use proc::*;

// Bundle state tracks start/stop operations.
#[derive(Debug, Eq, PartialEq)]
enum BundleState {
    Stopped,
    Running,
}

// We track the Bundle & ProcessControlInterface state.
struct BundleData {
    state: BundleState,
    bundle: Box<Bundle>,
}

impl BundleData {
    fn new(bundle: &Bundle) -> Self {
        BundleData {
            state: BundleState::Stopped,
            bundle: Box::new(*bundle),
        }
    }
}

// The ProcessManager presents the PackageManagementInterface (for loading
// applications from storage) and the ProcessControlInterface (for starting
// and stopping associated applications). The interface to the underlying
// system(s) are abstracted through the ProcessManagerInterface. One instance
// of the ProcessManager is created at start and accessed through SeL4 RPC's
// (from other components).
pub struct ProcessManager {
    manager: Box<dyn ProcessManagerInterface + Sync>,
    bundles: HashMap<BundleId, BundleData>,
}

impl ProcessManager {
    // Creates a new ProcessManager instance.
    pub fn new(manager: impl ProcessManagerInterface + Sync + 'static) -> ProcessManager {
        ProcessManager {
            manager: Box::new(manager),
            bundles: HashMap::with_capacity(DEFAULT_BUNDLES_CAPACITY),
        }
    }

    pub fn capacity(&self) -> usize {
        self.bundles.capacity()
    }
}

impl PackageManagementInterface for ProcessManager {
    fn install(&mut self, bundle_id: &str, bundle: &Bundle) -> Result<(), ProcessManagerError> {
        let bid = BundleId::from_str(bundle_id);
        if self.bundles.contains_key(&bid) {
            trace!("install {}: found", bundle_id);
            return Err(ProcessManagerError::BundleFound);
        }
        trace!("install {}", bundle_id);
        self.manager.install(bundle)?;
        assert!(self.bundles.insert(bid, BundleData::new(bundle)).is_none());
        Ok(())
    }

    fn uninstall(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        let bid = BundleId::from_str(bundle_id);
        // TODO(sleffler): the hashmap is ephemeral; should this always call
        // through to the storage manager?
        match self.bundles.remove(&bid) {
            None => {
                trace!("uninstall {}: not found", bundle_id);
                Err(ProcessManagerError::BundleNotFound)
            }
            Some(bundle) => {
                trace!("uninstall {}", bundle_id);
                self.manager.uninstall(&bundle.bundle)?;
                Ok(())
            }
        }
    }
}

impl ProcessControlInterface for ProcessManager {
    fn start(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        let bid = BundleId::from_str(bundle_id);
        match self.bundles.get_mut(&bid) {
            Some(bundle) => {
                trace!("start {}: state {:?}", bundle_id, bundle.state);
                if bundle.state == BundleState::Stopped {
                    self.manager.start(&bundle.bundle)?;
                }
                bundle.state = BundleState::Running;
                Ok(())
            }
            None => {
                // TODO(sleffler): the hashmap is ephemeral but the only
                // way atm to get a Bundle is with install?
                trace!("start {}: not found", bundle_id);
                Err(ProcessManagerError::BundleNotFound)
            }
        }
    }

    fn stop(&mut self, bundle_id: &str) -> Result<(), ProcessManagerError> {
        let bid = BundleId::from_str(bundle_id);
        match self.bundles.get_mut(&bid) {
            Some(bundle) => {
                trace!("stop {}: state {:?}", bundle_id, bundle.state);
                if bundle.state == BundleState::Running {
                    self.manager.stop(&bundle.bundle)?;
                }
                bundle.state = BundleState::Stopped;
                Ok(())
            }
            None => {
                trace!("stop {}: not found", bundle_id);
                // XXX ignore & return true?
                Err(ProcessManagerError::BundleNotFound)
            }
        }
    }

    fn get_running_bundles(&self) -> Result<BundleIdArray, ProcessManagerError> {
        trace!("get_running_bundles");
        let mut result = BundleIdArray::new();
        for (bundle_id, _bundle) in self
            .bundles
            .iter()
            .filter(|(_, bundle)| matches!(bundle.state, BundleState::Running))
        {
            result.push(bundle_id);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc::ProcessManagerError as pme;

    struct FakeManager;
    impl ProcessManagerInterface for FakeManager {
        fn install(&self, _bundle: &Bundle) -> Result<(), pme> {
            Ok(())
        }
        fn uninstall(&self, _bundle: &Bundle) -> Result<(), pme> {
            Ok(())
        }
        fn start(&self, _bundle: &Bundle) -> Result<(), pme> {
            Ok(())
        }
        fn stop(&self, _bundle: &Bundle) -> Result<(), pme> {
            Ok(())
        }
    }

    #[test]
    fn test_pkg_mgmt() {
        let fake = tests::FakeManager {};
        let mut mgr = ProcessManager::new(fake);

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
    fn test_spill() {
        let fake = tests::FakeManager {};
        let mut mgr = ProcessManager::new(fake);

        for i in 0..mgr.capacity() {
            let bundle_id = i.to_string();
            assert!(mgr.install(bundle_id.as_str(), &Bundle::new()).is_ok());
        }
        assert!(mgr.install("spill", &Bundle::new()).is_ok());
    }

    #[test]
    fn test_proc_ctrl() {
        let fake = tests::FakeManager {};
        let mut mgr = ProcessManager::new(fake);

        let bid2 = "2";
        let bid9 = "9";

        assert!(mgr.install(bid2, &Bundle::new()).is_ok());
        assert!(mgr.install(bid9, &Bundle::new()).is_ok());
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
