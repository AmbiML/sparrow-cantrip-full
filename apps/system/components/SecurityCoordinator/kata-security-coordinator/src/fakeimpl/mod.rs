//! Cantrip OS security coordinator fake support

extern crate alloc;
use alloc::fmt;
use alloc::string::{String, ToString};
use hashbrown::HashMap;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_memory_interface::cantrip_object_free_in_cnode;
use cantrip_security_interface::*;
use cantrip_storage_interface::KeyValueData;

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
        let _ = cantrip_object_free_in_cnode(&self.pkg_contents);
    }
}

pub struct FakeSecurityCoordinator {
    bundles: HashMap<String, BundleData>,
}
impl Default for FakeSecurityCoordinator {
    fn default() -> Self {
        Self::new()
    }
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
            .map_or_else(|| Err(SecurityRequestError::SreBundleNotFound), Ok)
    }
    fn get_bundle_mut(&mut self, bundle_id: &str) -> Result<&mut BundleData, SecurityRequestError> {
        self.bundles
            .get_mut(bundle_id)
            .map_or_else(|| Err(SecurityRequestError::SreBundleNotFound), Ok)
    }
    fn remove_bundle(&mut self, bundle_id: &str) -> Result<(), SecurityRequestError> {
        self.bundles
            .remove(bundle_id)
            .map_or_else(|| Err(SecurityRequestError::SreBundleNotFound), |_| Ok(()))
    }
}
pub type CantripSecurityCoordinatorInterface = FakeSecurityCoordinator;

impl SecurityCoordinatorInterface for FakeSecurityCoordinator {
    fn install(&mut self, pkg_contents: &ObjDescBundle) -> Result<String, SecurityRequestError> {
        // TODO(sleffler): get bundle_id from the manifest; for now use the
        //    cnode's CPtr since it is unique wrt all installed packages
        let bundle_id = fmt::format(format_args!("fake.{}", pkg_contents.cnode));
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
        let bundle = self.get_bundle_mut(bundle_id)?;
        // TODO(sleffler): error if no entry?
        let _ = bundle.keys.remove(key);
        Ok(())
    }
}
