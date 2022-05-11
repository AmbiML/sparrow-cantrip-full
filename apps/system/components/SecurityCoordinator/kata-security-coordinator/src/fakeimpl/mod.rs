//! Cantrip OS security coordinator fake support

extern crate alloc;
use alloc::string::{String, ToString};
use hashbrown::HashMap;
use cantrip_os_common::sel4_sys;
use cantrip_os_common::slot_allocator;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_memory_interface::cantrip_object_free;
use cantrip_security_interface::*;
use cantrip_storage_interface::KeyValueData;
use log::error;

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_WordBits;

use slot_allocator::CANTRIP_CSPACE_SLOTS;

extern "C" {
    static SELF_CNODE: seL4_CPtr;
}

struct BundleData {
    pkg_contents: ObjDescBundle,
    pkg_size: usize,
    manifest: String,
    keys: HashMap<String, KeyValueData>,
}
impl BundleData {
    fn new(pkg_contents: &ObjDescBundle) -> Self {
        let size = pkg_contents.objs.len() * 4096; // XXX
        BundleData {
            pkg_contents: pkg_contents.clone(),
            pkg_size: size,
            manifest: String::from(
                "# Comments like this
                        [Manifest]
                        BundleId=com.google.cerebra.hw.HelloWorld

                        [Binaries]
                        App=HelloWorldBin
                        Model=NeuralNetworkName

                        [Storage]
                        Required=1
                      ",
            ),
            keys: HashMap::with_capacity(2),
        }
    }
}
impl Drop for BundleData {
    fn drop(&mut self) {
        let _ = cantrip_object_free(&self.pkg_contents);
        unsafe {
            CANTRIP_CSPACE_SLOTS.free(self.pkg_contents.cnode, 1);
            if let Err(e) = seL4_CNode_Delete(SELF_CNODE, self.pkg_contents.cnode, seL4_WordBits as u8) {
                // XXX no bundle_id
                error!("Error deleting CNode {}, error {:?}", self.pkg_contents.cnode, e);
            }
        }
    }
}

pub struct FakeSecurityCoordinator {
    bundles: HashMap<String, BundleData>,
}
impl FakeSecurityCoordinator {
    pub fn new() -> Self {
        FakeSecurityCoordinator {
            bundles: HashMap::with_capacity(2),
        }
    }

    fn get_bundle(&self, bundle_id: &str) -> Result<&BundleData, SecurityRequestError> {
        self.bundles
            .get(bundle_id)
            .map_or_else(|| Err(SecurityRequestError::SreBundleNotFound), |v| Ok(v))
    }
    fn get_bundle_mut(&mut self, bundle_id: &str) -> Result<&mut BundleData, SecurityRequestError> {
        self.bundles
            .get_mut(bundle_id)
            .map_or_else(|| Err(SecurityRequestError::SreBundleNotFound), |v| Ok(v))
    }
    fn remove_bundle(&mut self, bundle_id: &str) -> Result<(), SecurityRequestError> {
        self.bundles
            .remove(bundle_id)
            .map_or_else(|| Err(SecurityRequestError::SreBundleNotFound), |_v| Ok(()))
    }
}
pub type CantripSecurityCoordinatorInterface = FakeSecurityCoordinator;

impl SecurityCoordinatorInterface for FakeSecurityCoordinator {
    fn install(&mut self, pkg_contents: &ObjDescBundle) -> Result<String, SecurityRequestError> {
        let bundle_id = "fubar".to_string(); // XXX
        if self.bundles.contains_key(&bundle_id) {
            return Err(SecurityRequestError::SreDeleteFirst);
        }
        assert!(self.bundles.insert(bundle_id.clone(), BundleData::new(pkg_contents)).is_none());
        Ok(bundle_id)
    }
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), SecurityRequestError> {
        self.remove_bundle(bundle_id)
    }
    fn size_buffer(&self, bundle_id: &str) -> Result<usize, SecurityRequestError> {
        let bundle = self.get_bundle(bundle_id)?;
        Ok(bundle.pkg_size) // TODO(sleffler): do better
    }
    fn get_manifest(&self, bundle_id: &str) -> Result<String, SecurityRequestError> {
        let bundle = self.get_bundle(bundle_id)?;
        // return &?
        Ok(bundle.manifest.clone())
    }
    fn load_application(&self, bundle_id: &str) -> Result<ObjDescBundle, SecurityRequestError> {
        let bundle_data = self.get_bundle(bundle_id)?;
        // XXX just return the package for now
        Ok(bundle_data.pkg_contents.clone())
    }
    fn load_model(&self, bundle_id: &str, _model_id: &str) -> Result<ObjDescBundle, SecurityRequestError> {
        let bundle_data = self.get_bundle(bundle_id)?;
        // TODO(sleffler): check model id
        // XXX just return the package for now
        Ok(bundle_data.pkg_contents.clone())
    }
    fn read_key(&self, bundle_id: &str, key: &str) -> Result<&KeyValueData, SecurityRequestError> {
        let bundle = self.get_bundle(bundle_id)?;
        bundle.keys.get(key).ok_or(SecurityRequestError::SreKeyNotFound)
    }
    fn write_key(&mut self, bundle_id: &str, key: &str, value: &KeyValueData) -> Result<(), SecurityRequestError> {
        let bundle = self.get_bundle_mut(bundle_id)?;
        let _ = bundle.keys.insert(key.to_string(), *value);
        Ok(())
    }
    fn delete_key(&mut self, bundle_id: &str, key: &str) -> Result<(), SecurityRequestError> {
        let bundle = self.get_bundle_mut(&bundle_id)?;
        // TODO(sleffler): error if no entry?
        let _ = bundle.keys.remove(key);
        Ok(())
    }
}
