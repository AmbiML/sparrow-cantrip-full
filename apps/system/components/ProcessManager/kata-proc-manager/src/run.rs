//! Cantrip OS process management support

// TODO(sleffler): need locking? (maybe guarantee single-threading via ipc)

#![cfg_attr(not(test), no_std)]
#![feature(const_fn_trait_bound)] // NB: for ProcessManager::empty using manager: None

#[cfg(not(test))]
extern crate cantrip_panic;

use arrayvec::ArrayVec;
use core::marker::Sync;
use log::debug;

use cantrip_allocator;
use cantrip_logger::CantripLogger;
use cantrip_proc_common as proc;
use proc::*;

static CANTRIP_LOGGER: CantripLogger = CantripLogger;

#[no_mangle]
pub extern "C" fn pre_init() {
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
}

#[no_mangle]
// NB: use post_init insted of pre_init so logger is setup
pub extern "C" fn post_init() {
    // TODO(sleffler): temp until we integrate with seL4
    static mut HEAP_MEMORY: [u8; 16 * 1024] = [0; 16 * 1024];
    unsafe {
        cantrip_allocator::ALLOCATOR.init(HEAP_MEMORY.as_mut_ptr() as usize, HEAP_MEMORY.len());
    }
}

// TODO(sleffler): move to init or similar if a thread isn't needed
#[no_mangle]
pub extern "C" fn run() {
    // Setup the userland address spaces, lifecycles, and system introspection
    // for third-party applications.
    debug!("run");
}

// Bundle state tracks start/stop operations.
#[derive(Debug, Eq, PartialEq)]
enum BundleState {
    Stopped,
    Running,
}

// We track the Bundle & ProcessControlInterface state.
// NB: assume storage manager (or other) owns Bundle
struct BundleData<'a> {
    state: BundleState,
    bundle: &'a Bundle,
}

impl<'b> BundleData<'b> {
    fn new(bundle: &'b Bundle) -> Self {
        BundleData {
            state: BundleState::Stopped,
            bundle: bundle,
        }
    }
}

// The ProcessManager presents the PackageManagementInterface (for loading
// applications from storage) and the ProcessControlInterface (for starting
// and stopping associated applications). The interface to the underlying
// system(s) are abstracted through the ProcessManagerInterface. One instance
// of the ProcessManager is created at start and accessed through SeL4 RPC's
// (from other components).
pub struct ProcessManager<'a> {
    // TODO(sleffler): Option is for empty which is meant for static setup
    manager: Option<&'a (dyn ProcessManagerInterface + Sync)>,

    // TODO(sleffler): hash table (requires missing deps)
    ids: ArrayVec<BundleId, MAX_BUNDLES>,
    bundles: ArrayVec<BundleData<'a>, MAX_BUNDLES>,
}

impl<'a> ProcessManager<'a> {
    // Creates a new ProcessManager instance.
    pub fn new(manager: &'a (dyn ProcessManagerInterface + Sync)) -> ProcessManager<'a> {
        ProcessManager {
            manager: Some(manager),
            ids: ArrayVec::<BundleId, MAX_BUNDLES>::new(),
            bundles: ArrayVec::<BundleData, MAX_BUNDLES>::new(),
        }
    }

    // Creates an incomplete ProcessManager instance for static initialization.
    // The instance must be followed with an init() call to complete setup.
    pub const fn empty() -> Self {
        ProcessManager {
            manager: None,
            ids: ArrayVec::<BundleId, MAX_BUNDLES>::new_const(),
            bundles: ArrayVec::<BundleData, MAX_BUNDLES>::new_const(),
        }
    }

    // Completes initialization of an instance created with empty().
    pub fn init(&mut self, manager: &'a (dyn ProcessManagerInterface + Sync)) {
        self.manager = Some(manager);
    }

    // Returns the index of |bundle_id| if previously installed.
    fn get_bundle_index(&self, bundle_id: &BundleId) -> Option<usize> {
        self.ids.iter().position(|x| bundle_id == x)
    }
}

impl<'a> PackageManagementInterface<'a> for ProcessManager<'a> {
    fn install(
        &mut self,
        bundle_id: &BundleId,
        bundle: &'a Bundle,
    ) -> Result<(), ProcessManagerError> {
        match self.get_bundle_index(bundle_id) {
            Some(_) => Err(ProcessManagerError::BundleFound),
            None => {
                if self.ids.is_full() {
                    return Err(ProcessManagerError::NoSpace);
                }
                self.manager.unwrap().install(bundle)?;
                self.bundles.push(BundleData::new(bundle));
                self.ids.push(*bundle_id);
                Ok(())
            }
        }
    }
    fn uninstall(&mut self, bundle_id: &BundleId) -> Result<(), ProcessManagerError> {
        match self.get_bundle_index(bundle_id) {
            None => Err(ProcessManagerError::BundleNotFound),
            Some(index) => {
                let bundle = &mut self.bundles[index];
                // TODO(sleffler): remove private state regardless of error?
                self.manager.unwrap().uninstall(bundle.bundle)?;
                self.bundles.remove(index);
                self.ids.remove(index);
                Ok(())
            }
        }
    }
}

impl<'a> ProcessControlInterface for ProcessManager<'a> {
    fn start(&mut self, bundle_id: &BundleId) -> Result<(), ProcessManagerError> {
        match self.get_bundle_index(bundle_id) {
            Some(index) => {
                let bundle = &mut self.bundles[index];
                if bundle.state == BundleState::Stopped {
                    self.manager.unwrap().start(bundle.bundle)?;
                }
                bundle.state = BundleState::Running;
                Ok(())
            }
            None => Err(ProcessManagerError::BundleNotFound),
        }
    }
    fn stop(&mut self, bundle_id: &BundleId) -> Result<(), ProcessManagerError> {
        match self.get_bundle_index(bundle_id) {
            Some(index) => {
                let bundle = &mut self.bundles[index];
                if bundle.state == BundleState::Running {
                    self.manager.unwrap().stop(bundle.bundle)?;
                }
                bundle.state = BundleState::Stopped;
                Ok(())
            }
            None => Err(ProcessManagerError::BundleNotFound), // XXX ignore & return true?
        }
    }
    fn get_running_bundles(&self) -> BundleIdArray {
        let mut result = BundleIdArray::new();
        for (index, (&id, _bundle)) in self
            .ids
            .iter()
            .zip(self.bundles.iter())
            .filter(|(_, bundle)| matches!(bundle.state, BundleState::Running))
            .enumerate()
        {
            result[index] = id;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc::ProcessManagerError as pme;

    struct FakeManager {}

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
        let bundle_id = BundleId::empty(1);
        let bundle = Bundle::new();
        let fake = tests::FakeManager {};
        let mut mgr = ProcessManager::new(&fake);

        // Not installed, should fail.
        assert_eq!(mgr.uninstall(&bundle_id).err(), Some(pme::BundleNotFound));
        // Install the bundle.
        assert!(mgr.install(&bundle_id, &bundle).is_ok());
        // Re-install the same bundle should fail.
        assert_eq!(
            mgr.install(&bundle_id, &bundle).err(),
            Some(pme::BundleFound)
        );
        // Now uninstalling the bundle should work.
        assert!(mgr.uninstall(&bundle_id).is_ok());
    }

    #[test]
    fn test_proc_ctrl() {
        let bundle_id = BundleId::empty(2);
        let bundle = Bundle::new();
        let fake = tests::FakeManager {};
        let mut mgr = ProcessManager::new(&fake);

        assert!(mgr.install(&bundle_id, &bundle).is_ok());
        assert!(mgr.stop(&bundle_id).is_ok());
        assert!(mgr.start(&bundle_id).is_ok());

        let running = mgr.get_running_bundles();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0], bundle_id);

        assert!(mgr.stop(&bundle_id).is_ok());
        // After stopping the bundle we should see nothing running.
        let running = mgr.get_running_bundles();
        assert_eq!(running.len(), 0);
    }

    #[test]
    fn test_empty_init() {
        let bundle_id = BundleId { id: [1; 32] };
        let bundle = Bundle::new();
        let fake = tests::FakeManager {};
        let mut mgr = ProcessManager::empty();
        mgr.init(&fake);

        // Not installed, should fail.
        assert_eq!(mgr.uninstall(&bundle_id).err(), Some(pme::BundleNotFound));
        // Install the bundle.
        assert!(mgr.install(&bundle_id, &bundle).is_ok());
        // Re-install the same bundle should fail.
        assert_eq!(
            mgr.install(&bundle_id, &bundle).err(),
            Some(pme::BundleFound)
        );
        // Now uninstalling the bundle should work.
        assert!(mgr.uninstall(&bundle_id).is_ok());
    }
}
