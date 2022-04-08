//! Cantrip OS security coordinator fake support

extern crate alloc;
use alloc::string::{String, ToString};
use hashbrown::HashMap;
use cantrip_security_interface::DeleteKeyRequest;
use cantrip_security_interface::GetManifestRequest;
use cantrip_security_interface::LoadApplicationRequest;
use cantrip_security_interface::LoadModelRequest;
use cantrip_security_interface::ReadKeyRequest;
use cantrip_security_interface::SecurityCoordinatorInterface;
use cantrip_security_interface::SecurityRequest;
use cantrip_security_interface::SecurityRequestError;
use cantrip_security_interface::SizeBufferRequest;
use cantrip_security_interface::UninstallRequest;
use cantrip_security_interface::WriteKeyRequest;
use cantrip_storage_interface::KeyValueData;
use cantrip_storage_interface::KEY_VALUE_DATA_SIZE;
use log::trace;
use postcard;

struct BundleData {
    pkg_size: usize,
    manifest: String,
    keys: HashMap<String, KeyValueData>,
}
impl BundleData {
    fn new(pkg_size: usize) -> Self {
        BundleData {
            pkg_size: pkg_size,
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
    fn request(
        &mut self,
        request_id: SecurityRequest,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> Result<(), SecurityRequestError> {
        use SecurityRequestError::*;

        fn serialize_failure(e: postcard::Error) -> SecurityRequestError {
            trace!("serialize failed: {:?}", e);
            SreBundleDataInvalid
        }
        fn deserialize_failure(e: postcard::Error) -> SecurityRequestError {
            trace!("deserialize failed: {:?}", e);
            SreBundleDataInvalid
        }

        // TODO(sleffler): mailbox ipc
        match request_id {
            SecurityRequest::SrEcho => {
                trace!("ECHO {:?}", request_buffer);
                reply_buffer[0..request_buffer.len()].copy_from_slice(&request_buffer[..]);
                Ok(())
            }
            SecurityRequest::SrInstall => {
                trace!(
                    "INSTALL addr {:p} len {}",
                    request_buffer.as_ptr(),
                    request_buffer.len()
                );
                //                let bundle_id = (request_buffer.as_ptr() as usize).to_string();
                // TODO(sleffler): used by cantrip-storage-component for kvops
                let bundle_id = "fubar".to_string();
                let _ = postcard::to_slice(&bundle_id, reply_buffer).map_err(serialize_failure)?;
                assert!(self
                    .bundles
                    .insert(bundle_id, BundleData::new(request_buffer.len()))
                    .is_none());
                Ok(())
            }
            SecurityRequest::SrUninstall => {
                let request = postcard::from_bytes::<UninstallRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("UNINSTALL {}", request.bundle_id);
                self.remove_bundle(&request.bundle_id)
            }
            SecurityRequest::SrSizeBuffer => {
                let request = postcard::from_bytes::<SizeBufferRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("SIZE BUFFER bundle_id {}", request.bundle_id);
                let bundle = self.get_bundle(&request.bundle_id)?;
                let _ = postcard::to_slice(
                    &bundle.pkg_size, // TODO(sleffler): do better
                    reply_buffer,
                )
                .map_err(serialize_failure)?;
                Ok(())
            }
            SecurityRequest::SrGetManifest => {
                let request = postcard::from_bytes::<GetManifestRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("GET MANIFEST bundle_id {}", request.bundle_id);
                let bundle = self.get_bundle(&request.bundle_id)?;
                let _ = postcard::to_slice(&bundle.manifest, reply_buffer)
                    .map_err(serialize_failure)?;
                Ok(())
            }
            SecurityRequest::SrLoadApplication => {
                let request = postcard::from_bytes::<LoadApplicationRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!(
                    "LOAD APPLICATION bundle_id {} addr {:p}",
                    request.bundle_id,
                    request.app_binary
                );
                let _ = self.get_bundle(&request.bundle_id)?;
                Ok(())
            }
            SecurityRequest::SrLoadModel => {
                let request = postcard::from_bytes::<LoadModelRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!(
                    "LOAD MODEL bundle_id {} model_id {} addr {:p}",
                    request.bundle_id,
                    request.model_id,
                    request.model_binary
                );
                // TODO(sleffler): check model id
                let _ = self.get_bundle(&request.bundle_id)?;
                Ok(())
            }
            SecurityRequest::SrReadKey => {
                let request = postcard::from_bytes::<ReadKeyRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!(
                    "READ KEY bundle_id {} key {}",
                    request.bundle_id,
                    request.key,
                );
                let bundle = self.get_bundle(&request.bundle_id)?;
                match bundle.keys.get(request.key) {
                    Some(value) => {
                        // TODO(sleffler): return values are fixed size unless we serialize
                        reply_buffer[..value.len()].copy_from_slice(&value[..]);
                        Ok(())
                    }
                    None => Err(SreKeyNotFound),
                }
            }
            SecurityRequest::SrWriteKey => {
                let request = postcard::from_bytes::<WriteKeyRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!(
                    "WRITE KEY bundle_id {} key {} value {:?}",
                    request.bundle_id,
                    request.key,
                    request.value,
                );
                let bundle = self.get_bundle_mut(&request.bundle_id)?;
                // TODO(sleffler): optimnize with entry
                let mut value = [0u8; KEY_VALUE_DATA_SIZE];
                value[..request.value.len()].copy_from_slice(request.value);
                let _ = bundle.keys.insert(request.key.to_string(), value);
                Ok(())
            }
            SecurityRequest::SrDeleteKey => {
                let request = postcard::from_bytes::<DeleteKeyRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!(
                    "DELETE KEY bundle_id {} key {}",
                    request.bundle_id,
                    request.key,
                );
                let bundle = self.get_bundle_mut(&request.bundle_id)?;
                // TODO(sleffler): error if no entry?
                let _ = bundle.keys.remove(request.key);
                Ok(())
            }
        }
    }
}
