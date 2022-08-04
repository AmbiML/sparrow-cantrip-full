//! Cantrip OS security coordinator seL4 support

use cantrip_security_interface::DeleteKeyRequest;
use cantrip_security_interface::GetManifestRequest;
use cantrip_security_interface::LoadApplicationRequest;
use cantrip_security_interface::LoadModelRequest;
use cantrip_security_interface::ReadKeyRequest;
use cantrip_security_interface::SecurityCoordinatorInterface;
use cantrip_security_interface::SecurityRequest;
use cantrip_security_interface::SecurityRequestCapability;
use cantrip_security_interface::SecurityRequestError;
use cantrip_security_interface::SizeBufferRequest;
use cantrip_security_interface::UninstallRequest;
use cantrip_security_interface::WriteKeyRequest;
use log::trace;
use postcard;

extern "C" {
    static SECURITY_RECV_SLOT: seL4_CPtr;
}

pub struct SeL4SecurityCoordinator {
    // TODO(sleffler): mailbox ipc state
}
impl SeL4SecurityCoordinator {
    pub fn new() -> Self { SeL4SecurityCoordinator {} }
}
pub type CantripSecurityCoordinatorInterface = SeL4SecurityCoordinator;

impl SecurityCoordinatorInterface for SeL4SecurityCoordinator {
    fn request(
        &mut self,
        request_id: SecurityRequest,
        request_buffer: &[u8],
        _reply_buffer: &mut [u8],
    ) -> Result<(), SecurityRequestError> {
        use SecurityRequestError::*;

        fn _serialize_failure(e: postcard::Error) -> SecurityRequestError {
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
                // TODO(sleffler): fill-in
                Err(SreEchoFailed)
            }
            SecurityRequest::SrInstall => {
                let mut request = postcard::from_bytes::<InstallRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                request.set_container_cap(unsafe { SECURITY_RECV_SLOT });
                trace!("INSTALL pkg_contents {:?}", request.pkg_contents);
                // TODO(sleffler): fill-in
                Err(SreInstallFailed)
            }
            SecurityRequest::SrUninstall => {
                let request = postcard::from_bytes::<UninstallRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("UNINSTALL {}", request.bundle_id);
                // TODO(sleffler): fill-in
                Err(SreUninstallFailed)
            }
            SecurityRequest::SrSizeBuffer => {
                let request = postcard::from_bytes::<SizeBufferRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("SIZE BUFFER bundle_id {}", request.bundle_id);
                // TODO(sleffler): fill-in
                Err(SreSizeBufferFailed)
            }
            SecurityRequest::SrGetManifest => {
                let request = postcard::from_bytes::<GetManifestRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("GET MANIFEST bundle_id {}", request.bundle_id);
                // TODO(sleffler): fill-in
                Err(SreGetManifestFailed)
            }
            SecurityRequest::SrLoadApplication => {
                let mut request =
                    postcard::from_bytes::<LoadApplicationRequest>(&request_buffer[..])
                        .map_err(deserialize_failure)?;
                request.set_container_cap(unsafe { SECURITY_RECV_SLOT });
                trace!(
                    "LOAD APPLICATION bundle_id {} app_binary {:?}",
                    request.bundle_id,
                    request.app_binary
                );
                // TODO(sleffler): fill-in
                Err(SreLoadApplicationFailed)
            }
            SecurityRequest::SrLoadModel => {
                let mut request = postcard::from_bytes::<LoadModelRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                request.set_container_cap(unsafe { SECURITY_RECV_SLOT });
                trace!(
                    "LOAD MODEL bundle_id {} model_id {} model_binary {:?}",
                    request.bundle_id,
                    request.model_id,
                    request.model_binary
                );
                // TODO(sleffler): fill-in
                Err(SreLoadModelFailed)
            }
            SecurityRequest::SrReadKey => {
                let request = postcard::from_bytes::<ReadKeyRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("READ KEY bundle_id {} key {}", request.bundle_id, request.key,);
                // TODO(sleffler): fill-in
                Err(SreReadFailed)
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
                // TODO(sleffler): fill-in
                Err(SreWriteFailed)
            }
            SecurityRequest::SrDeleteKey => {
                let request = postcard::from_bytes::<DeleteKeyRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("DELETE KEY bundle_id {} key {}", request.bundle_id, request.key,);
                // TODO(sleffler): fill-in
                Err(SreDeleteFailed)
            }
        }
    }
}
