//! Cantrip OS security coordinator seL4 support

extern crate alloc;
use cantrip_security_common::*;
use log::trace;
use postcard;

pub struct SeL4SecurityCoordinatorInterface {
    // TODO(sleffler): mailbox ipc state
}
impl SeL4SecurityCoordinatorInterface {
    pub fn new() -> Self {
        SeL4SecurityCoordinatorInterface {}
    }
}
pub type CantripSecurityCoordinatorInterface = SeL4SecurityCoordinatorInterface;

impl SecurityCoordinatorInterface for SeL4SecurityCoordinatorInterface {
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
                trace!(
                    "INSTALL addr {:p} len {}",
                    request_buffer.as_ptr(),
                    request_buffer.len()
                );
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
                let request = postcard::from_bytes::<SizeBufferRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("GET MANIFEST bundle_id {}", request.bundle_id);
                // TODO(sleffler): fill-in
                Err(SreGetManifestFailed)
            }
            SecurityRequest::SrLoadApplication => {
                let request = postcard::from_bytes::<LoadApplicationRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!(
                    "LOAD APPLICATION bundle_id {} addr {:p}",
                    request.bundle_id,
                    request.app_binary
                );
                // TODO(sleffler): fill-in
                Err(SreLoadApplicationFailed)
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
                // TODO(sleffler): fill-in
                Err(SreLoadModelFailed)
            }
            SecurityRequest::SrReadKey => {
                let request = postcard::from_bytes::<ReadKeyRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!(
                    "READ KEY bundle_id {} key {}",
                    request.bundle_id,
                    request.key,
                );
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
                trace!(
                    "DELETE KEY bundle_id {} key {}",
                    request.bundle_id,
                    request.key,
                );
                // TODO(sleffler): fill-in
                Err(SreDeleteFailed)
            }
        }
    }
}
