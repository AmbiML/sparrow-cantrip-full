//! Cantrip OS security coordinator fake support

extern crate alloc;
use alloc::string::ToString;
use cantrip_security_common::*;
use log::trace;
use postcard;

pub struct FakeSecurityCoordinatorInterface {
    // TODO(sleffler): mailbox ipc state
}
impl FakeSecurityCoordinatorInterface {
    pub fn new() -> Self {
        FakeSecurityCoordinatorInterface {}
    }
}
pub type CantripSecurityCoordinatorInterface = FakeSecurityCoordinatorInterface;

impl SecurityCoordinatorInterface for FakeSecurityCoordinatorInterface {
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
                let _ = postcard::to_slice(
                    &(request_buffer.as_ptr() as usize).to_string(),
                    reply_buffer,
                )
                .map_err(serialize_failure)?;
                Ok(())
            }
            SecurityRequest::SrUninstall => {
                let request = postcard::from_bytes::<UninstallRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("UNINSTALL {}", request.bundle_id);
                Ok(())
            }
            SecurityRequest::SrSizeBuffer => {
                let request = postcard::from_bytes::<SizeBufferRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("SIZE BUFFER bundle_id {}", request.bundle_id);
                let _ = postcard::to_slice(
                    &0u32, // TODO(sleffler): fill-in
                    reply_buffer,
                )
                .map_err(serialize_failure)?;
                Ok(())
            }
            SecurityRequest::SrGetManifest => {
                let request = postcard::from_bytes::<SizeBufferRequest>(&request_buffer[..])
                    .map_err(deserialize_failure)?;
                trace!("GET MANIFEST bundle_id {}", request.bundle_id);
                let _ = postcard::to_slice(
                    "# Comments like this
                        [Manifest]
                        BundleId=com.google.cerebra.hw.HelloWorld

                        [Binaries]
                        App=HelloWorldBin
                        Model=NeuralNetworkName

                        [Storage]
                        Required=1
                        ", // TODO(sleffler): fill-in
                    reply_buffer,
                )
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
