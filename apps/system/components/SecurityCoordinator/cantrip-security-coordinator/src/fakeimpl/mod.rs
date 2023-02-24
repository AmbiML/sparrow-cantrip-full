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

//! Cantrip OS security coordinator fake support

extern crate alloc;
use crate::upload::Upload;
use alloc::fmt;
use alloc::string::{String, ToString};
use cantrip_memory_interface::cantrip_cnode_alloc;
use cantrip_memory_interface::cantrip_object_free_in_cnode;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::copyregion::CopyRegion;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use cantrip_security_interface::*;
use core::mem::size_of;
use core::ptr;
use hashbrown::HashMap;
use log::info;

use sel4_sys::seL4_Error;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_Word;

const PAGE_SIZE: usize = 1 << seL4_PageBits;

extern "C" {
    // Regions for deep_copy work.
    static mut DEEP_COPY_SRC: [seL4_Word; PAGE_SIZE / size_of::<seL4_Word>()];
    static mut DEEP_COPY_DEST: [seL4_Word; PAGE_SIZE / size_of::<seL4_Word>()];
}

struct BundleData {
    pkg_contents: ObjDescBundle,
    pkg_size: usize,
    manifest: String,
    keys: HashMap<String, KeyValueData>,
}
impl BundleData {
    fn new(pkg_contents: &ObjDescBundle) -> Self {
        BundleData {
            pkg_contents: pkg_contents.clone(),
            pkg_size: pkg_contents.size_bytes(),
            manifest: String::from(
                r##"
# Comments like this
[Manifest]
BundleId=com.google.cerebra.hw.HelloWorld

[Binaries]
App=HelloWorldBin
Model=NeuralNetworkName

[Storage]
Required=1
"##,
            ),
            keys: HashMap::with_capacity(2),
        }
    }
}
impl Drop for BundleData {
    fn drop(&mut self) { let _ = cantrip_object_free_in_cnode(&self.pkg_contents); }
}

pub struct FakeSecurityCoordinator {
    bundles: HashMap<String, BundleData>,
}
impl Default for FakeSecurityCoordinator {
    fn default() -> Self { Self::new() }
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

// Returns a deep copy (including seL4 objects) of |src|. The container
// CNode is in the toplevel (allocated with the slot allocator).
fn deep_copy(src: &ObjDescBundle) -> Result<ObjDescBundle, seL4_Error> {
    // Dest is an upload object that allocates a page at-a-time so
    // the MemoryManager doesn't have to handle a huge memory request.
    let mut dest = Upload::new(unsafe { ptr::addr_of_mut!(DEEP_COPY_DEST[0]) }, PAGE_SIZE);

    // Src top-level slot & copy region
    let src_slot = CSpaceSlot::new();
    let mut src_region = unsafe { CopyRegion::new(ptr::addr_of_mut!(DEEP_COPY_SRC[0]), PAGE_SIZE) };

    for src_cptr in src.cptr_iter() {
        // Map src frame and copy data (allocating memory as needed)..
        src_slot
            .dup_to(src.cnode, src_cptr, src.depth)
            .and_then(|_| src_region.map(src_slot.slot))?;
        dest.write(src_region.as_ref())
            .or(Err(seL4_Error::seL4_NotEnoughMemory))?; // TODO(sleffler) From mapping

        // Unmap & clear top-level src slot required for mapping.
        src_region.unmap().and_then(|_| src_slot.delete())?;
    }
    dest.finish();

    // Collect the frames in a top-level CNode.
    let cnode_depth = dest.frames().count_log2();
    let cnode = cantrip_cnode_alloc(cnode_depth).or(Err(seL4_Error::seL4_NotEnoughMemory))?; // TODO(sleffler) From mapping
    dest.frames_mut()
        .move_objects_from_toplevel(cnode.objs[0].cptr, cnode_depth as u8)?;
    Ok(dest.frames().clone())
}

impl SecurityCoordinatorInterface for FakeSecurityCoordinator {
    fn install(&mut self, pkg_contents: &ObjDescBundle) -> Result<String, SecurityRequestError> {
        // TODO(sleffler): get bundle_id from the manifest; for now use the
        //    cnode's CPtr since it is unique wrt all installed packages
        let bundle_id = fmt::format(format_args!("fake.{}", pkg_contents.cnode));
        if self.bundles.contains_key(&bundle_id) {
            return Err(SecurityRequestError::SreDeleteFirst);
        }
        assert!(self
            .bundles
            .insert(bundle_id.clone(), BundleData::new(pkg_contents))
            .is_none());
        Ok(bundle_id)
    }
    fn install_app(
        &mut self,
        app_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), SecurityRequestError> {
        if self.bundles.contains_key(app_id) {
            return Err(SecurityRequestError::SreDeleteFirst);
        }
        assert!(self
            .bundles
            .insert(app_id.into(), BundleData::new(pkg_contents))
            .is_none());
        Ok(())
    }
    fn install_model(
        &mut self,
        _app_id: &str,
        model_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), SecurityRequestError> {
        if self.bundles.contains_key(model_id) {
            return Err(SecurityRequestError::SreDeleteFirst);
        }
        assert!(self
            .bundles
            .insert(model_id.into(), BundleData::new(pkg_contents))
            .is_none());
        Ok(())
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
        // Clone everything (struct + associated seL4 objects) so the
        // return is as though it was newly instantiated from flash.
        // XXX just return the package for now
        deep_copy(&bundle_data.pkg_contents).or(Err(SecurityRequestError::SreLoadApplicationFailed))
    }
    fn load_model(
        &self,
        _bundle_id: &str,
        model_id: &str,
    ) -> Result<ObjDescBundle, SecurityRequestError> {
        // TODO(sleffler): models are meant to be associated with bundle_id
        let model_data = self.get_bundle(model_id)?;
        // Clone everything (struct + associated seL4 objects) so the
        // return is as though it was newly instantiated from flash.
        deep_copy(&model_data.pkg_contents).or(Err(SecurityRequestError::SreLoadModelFailed))
    }
    fn read_key(&self, bundle_id: &str, key: &str) -> Result<&KeyValueData, SecurityRequestError> {
        let bundle = self.get_bundle(bundle_id)?;
        bundle
            .keys
            .get(key)
            .ok_or(SecurityRequestError::SreKeyNotFound)
    }
    fn write_key(
        &mut self,
        bundle_id: &str,
        key: &str,
        value: &KeyValueData,
    ) -> Result<(), SecurityRequestError> {
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

    fn test_mailbox(&mut self) -> Result<(), SecurityRequestError> {
        info!("This is a fake with no mailbox api");
        Err(SecurityRequestError::SreTestFailed)
    }
}
