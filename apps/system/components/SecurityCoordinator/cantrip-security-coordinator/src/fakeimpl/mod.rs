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

//! Cantrip OS Security Coordinator fake support.
//! Reads directly from simulated flash and holds new data in dynamically
//! allocated memory (so it's not preserved across restarts). The flash
//! is a replacement for talking to the Security Core where the flash is
//! located on real hardware. And the fake returns package contents
//! any verification (TBD by the Security Core).

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
use cpio::CpioNewcReader;
use hashbrown::HashMap;
use log::{error, info};

use sel4_sys::seL4_Error;

const CAPACITY_BUNDLES: usize = 10; // HashMap of bundles
const CAPACITY_KEYS: usize = 2; // Per-bundle HashMap of key-values

const APP_SUFFIX: &str = ".app";
const MODEL_SUFFIX: &str = ".model";

const FAKE_APP_MANIFEST: &str = r##"
# Comments like this
[Manifest]
BundleId=com.google.cerebra.hw.HelloWorld

[Binaries]
App=HelloWorldBin
Model=NeuralNetworkName

[Storage]
Required=1
"##;

extern "Rust" {
    fn get_cpio_archive() -> &'static [u8]; // CPIO archive of built-in files

    // Regions for deep_copy work.
    fn get_deep_copy_src_mut() -> &'static mut [u8];
    fn get_deep_copy_dest_mut() -> &'static mut [u8];
}

/// Package contents either come from built-in files or dynamically
/// loaded from the DebugConsole. Builtin package data resides in simulated
/// Flash. Dynamically loaded package data is stored in memory obtained from
/// the MemoryManager.
enum PkgContents {
    Flash(&'static [u8]),   // Data resides in simulated Flash
    Dynamic(ObjDescBundle), // Data resides in dynamically allocated memory
}

struct BundleData {
    pkg_contents: PkgContents,
    pkg_size: usize,
    manifest: String, // XXX not used
    keys: HashMap<String, KeyValueData>,
}
impl BundleData {
    // Returns a bundle for a dynamically loaded package.
    fn new(pkg_contents: &ObjDescBundle, manifest: &str) -> Self {
        Self {
            pkg_contents: PkgContents::Dynamic(pkg_contents.clone()),
            pkg_size: pkg_contents.size_bytes(),
            manifest: manifest.to_string(),
            keys: HashMap::with_capacity(CAPACITY_KEYS),
        }
    }

    // Returns a bundle for a builtin package.
    fn new_from_flash(slice: &'static [u8], manifest: &str) -> Self {
        Self {
            pkg_contents: PkgContents::Flash(slice),
            pkg_size: slice.len(),
            manifest: manifest.to_string(),
            keys: HashMap::with_capacity(CAPACITY_KEYS),
        }
    }

    // Returns a copy of the package contents suitable for sending
    // to another thread. The data are copied to newly allocated frames
    // and the frames are aggregated in a CNode ready to attach to
    // an IPC message.
    fn deep_copy(&self) -> Result<ObjDescBundle, seL4_Error> {
        let mut upload = match &self.pkg_contents {
            PkgContents::Flash(data) => upload_slice(data),
            PkgContents::Dynamic(bundle) => upload_obj_bundle(bundle),
        }?;

        // Collect the frames in a top-level CNode.
        let cnode_depth = upload.frames().count_log2();
        let cnode =
            cantrip_cnode_alloc(cnode_depth).map_err(|_| seL4_Error::seL4_NotEnoughMemory)?; // TODO(sleffler) From mapping
        upload
            .frames_mut()
            .move_objects_from_toplevel(cnode.objs[0].cptr, cnode_depth as u8)?;
        Ok(upload.frames().clone())
    }
}
impl Drop for BundleData {
    fn drop(&mut self) {
        if let PkgContents::Dynamic(bundle) = &self.pkg_contents {
            let _ = cantrip_object_free_in_cnode(bundle);
        }
    }
}

// Returns an array of bundle id's from the builtin archive.
fn get_builtins() -> BundleIdArray {
    let mut builtins = BundleIdArray::new();
    for e in CpioNewcReader::new(unsafe { get_cpio_archive() }) {
        if e.is_err() {
            error!("cpio read err {:?}", e);
            break;
        }
        builtins.push(e.unwrap().name.to_string());
    }
    builtins
}

// Returns a bundle backed by builtin data.
fn get_bundle_from_builtins(filename: &str) -> Result<BundleData, SecurityRequestError> {
    fn builtins_lookup(filename: &str) -> Option<&'static [u8]> {
        for e in CpioNewcReader::new(unsafe { get_cpio_archive() }) {
            if e.is_err() {
                error!("cpio read err {:?}", e);
                break;
            }
            let entry = e.unwrap();
            if entry.name == filename {
                return Some(entry.data);
            }
        }
        None
    }
    builtins_lookup(filename)
        .ok_or(SecurityRequestError::SreBundleNotFound)
        .map(|data| BundleData::new_from_flash(data, ""))
}

// Returns a copy (including seL4 objects) of |src| in an Upload container.
fn upload_obj_bundle(src: &ObjDescBundle) -> Result<Upload, seL4_Error> {
    // Dest is an upload object that allocates a page at-a-time so
    // the MemoryManager doesn't have to handle a huge memory request.
    let mut dest = Upload::new(unsafe { get_deep_copy_dest_mut() });

    // Src top-level slot & copy region
    let src_slot = CSpaceSlot::new();
    let mut src_region = unsafe { CopyRegion::new(get_deep_copy_src_mut()) };

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
    Ok(dest)
}

// Returns a copy (including seL4 objects) of |src| in an Upload container.
fn upload_slice(src: &[u8]) -> Result<Upload, seL4_Error> {
    // Dest is an upload object that allocates a page at-a-time so
    // the MemoryManager doesn't have to handle a huge memory request.
    let mut dest = Upload::new(unsafe { get_deep_copy_dest_mut() });
    dest.write(src).or(Err(seL4_Error::seL4_NotEnoughMemory))?;
    dest.finish();
    Ok(dest)
}

// Returns |key| or |key|+|suffix| if |key| does not end with |suffix|.
fn promote_key(key: &str, suffix: &str) -> String {
    if key.ends_with(suffix) {
        key.to_string()
    } else {
        key.to_string() + suffix
    }
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
            bundles: HashMap::with_capacity(CAPACITY_BUNDLES),
        }
    }

    // Probes for a bundle named |key| or |key|+<suffix>; returning Some(v)
    // where |v| is the key under which the bundle is registered.
    fn find_key(&self, key: &str) -> Option<String> {
        if self.bundles.contains_key(key) {
            Some(key.to_string())
        } else if self.bundles.contains_key(&(key.to_string() + APP_SUFFIX)) {
            Some(key.to_string() + APP_SUFFIX)
        } else if self.bundles.contains_key(&(key.to_string() + MODEL_SUFFIX)) {
            Some(key.to_string() + MODEL_SUFFIX)
        } else {
            None
        }
    }

    // Returns a ref for |bundle_id|'s entry.
    fn get_bundle(&self, bundle_id: &str) -> Result<&BundleData, SecurityRequestError> {
        self.find_key(bundle_id)
            .and_then(|key| self.bundles.get(&key))
            .ok_or(SecurityRequestError::SreBundleNotFound)
    }
    // Returns a mutable ref for |bundle_id|'s entry.
    fn get_bundle_mut(&mut self, bundle_id: &str) -> Result<&mut BundleData, SecurityRequestError> {
        self.find_key(bundle_id)
            .and_then(|key| self.bundles.get_mut(&key))
            .ok_or(SecurityRequestError::SreBundleNotFound)
    }

    // Remove any entry for |bundle_id|.
    fn remove_bundle(&mut self, bundle_id: &str) -> Result<(), SecurityRequestError> {
        self.find_key(bundle_id)
            .and_then(|key| self.bundles.remove(&key))
            .map_or_else(|| Err(SecurityRequestError::SreBundleNotFound), |_| Ok(()))
    }

    // Returns a ref for |bundle_id|'s entry, possibly instantiating a
    // new entry using builtin package contents if no entry exists.
    fn get_bundle_or_builtin(
        &mut self,
        bundle_id: &str,
        suffix: &str,
    ) -> Result<&BundleData, SecurityRequestError> {
        if self.bundles.contains_key(bundle_id) {
            return self.get_bundle(bundle_id);
        }
        if let Ok(bd) = get_bundle_from_builtins(bundle_id) {
            assert!(self.bundles.insert(bundle_id.to_string(), bd).is_none());
            return self.get_bundle(bundle_id);
        }
        let key = promote_key(bundle_id, suffix);
        if !self.bundles.contains_key(&key) {
            let bd = get_bundle_from_builtins(&key)?;
            assert!(self.bundles.insert(key.clone(), bd).is_none());
        }
        self.get_bundle(&key) // XXX self.bundles.get to avoid find_key
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
        assert!(self
            .bundles
            .insert(bundle_id.clone(), BundleData::new(pkg_contents, ""))
            .is_none());
        Ok(bundle_id)
    }
    fn install_app(
        &mut self,
        app_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), SecurityRequestError> {
        let key = promote_key(app_id, APP_SUFFIX);
        if self.bundles.contains_key(&key) {
            return Err(SecurityRequestError::SreDeleteFirst);
        }
        assert!(self
            .bundles
            .insert(key, BundleData::new(pkg_contents, FAKE_APP_MANIFEST))
            .is_none());
        Ok(())
    }
    fn install_model(
        &mut self,
        _app_id: &str,
        model_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), SecurityRequestError> {
        let key = promote_key(model_id, MODEL_SUFFIX);
        if self.bundles.contains_key(&key) {
            return Err(SecurityRequestError::SreDeleteFirst);
        }
        assert!(self
            .bundles
            .insert(key, BundleData::new(pkg_contents, ""))
            .is_none());
        Ok(())
    }
    fn uninstall(&mut self, bundle_id: &str) -> Result<(), SecurityRequestError> {
        // NB: does not remove flash/built-in contents
        self.remove_bundle(bundle_id)
    }

    fn get_packages(&self) -> Result<BundleIdArray, SecurityRequestError> {
        // First, dynamically installed bundles.
        let mut result: BundleIdArray = self.bundles.keys().cloned().collect();
        // Second, builtins.
        result.append(&mut get_builtins());
        result.sort();
        result.dedup();
        Ok(result)
    }

    // TODO(sleffler): use get_bundle so package must be loaded? instantiating
    //   hashmap entries may be undesirable
    fn size_buffer(&self, bundle_id: &str) -> Result<usize, SecurityRequestError> {
        let bundle = self.get_bundle(bundle_id)?;
        Ok(bundle.pkg_size) // TODO(sleffler): do better
    }
    fn get_manifest(&self, bundle_id: &str) -> Result<String, SecurityRequestError> {
        let bundle = self.get_bundle(bundle_id)?;
        Ok(bundle.manifest.clone())
    }

    // NB: loading may promote a bundle from the built-ins archive to the hashmap
    fn load_application(&mut self, bundle_id: &str) -> Result<ObjDescBundle, SecurityRequestError> {
        let bundle_data = self.get_bundle_or_builtin(bundle_id, APP_SUFFIX)?;
        // Clone everything (struct + associated seL4 objects) so the
        // return is as though it was newly instantiated from flash.
        // XXX just return the package for now
        bundle_data
            .deep_copy()
            .or(Err(SecurityRequestError::SreLoadApplicationFailed))
    }
    fn load_model(
        &mut self,
        _bundle_id: &str, // TODO(sleffler): models are meant to be associated with bundle_id
        model_id: &str,
    ) -> Result<ObjDescBundle, SecurityRequestError> {
        let model_data = self.get_bundle_or_builtin(model_id, MODEL_SUFFIX)?;
        // Clone everything (struct + associated seL4 objects) so the
        // return is as though it was newly instantiated from flash.
        model_data
            .deep_copy()
            .or(Err(SecurityRequestError::SreLoadModelFailed))
    }

    // NB: key-value ops require a load'd bundle so only do get_bundle
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
        value: &[u8],
    ) -> Result<(), SecurityRequestError> {
        let bundle = self.get_bundle_mut(bundle_id)?;
        let mut keyval = [0u8; KEY_VALUE_DATA_SIZE];
        keyval[..value.len()].copy_from_slice(value);
        let _ = bundle.keys.insert(key.to_string(), keyval);
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
