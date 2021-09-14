//! Cantrip OS security coordinator support

#![cfg_attr(not(test), no_std)]
// NB: "error[E0658]: trait bounds other than `Sized` on const fn parameters are unstable"
#![feature(const_fn_trait_bound)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::ToString;
use cantrip_security_common::*;
use log::trace;
use postcard;

#[cfg(not(test))]
pub static mut CANTRIP_SECURITY: CantripSecurityCoordinator = CantripSecurityCoordinator::empty();

// CantripSecurityCoordinator bundles an instance of the SecurityCoordinator that operates
// on CantripOS interfaces. There is a two-step dance to setup an instance because we want
// CANTRIP_STORAGE static.
// NB: no locking is done; we assume the caller/user is single-threaded
pub struct CantripSecurityCoordinator {
    manager: Option<Box<dyn SecurityCoordinatorInterface + Sync>>,
    // TODO(sleffler): mailbox ipc state
}
impl CantripSecurityCoordinator {
    // Constructs a partially-initialized instance; to complete call init().
    // This is needed because we need a const fn for static setup.
    const fn empty() -> CantripSecurityCoordinator {
        CantripSecurityCoordinator { manager: None }
    }

    pub fn init(&mut self) {
        self.manager = Some(Box::new(CantripSecurityCoordinatorInterface));
    }
}
impl SecurityCoordinatorInterface for CantripSecurityCoordinator {
    fn request(
        &mut self,
        request_id: SecurityRequest,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> Result<(), SecurityRequestError> {
        self.manager
            .as_mut()
            .unwrap()
            .request(request_id, request_buffer, reply_buffer)
    }
}

struct CantripSecurityCoordinatorInterface;
// TODO(sleffler): move this to a feature-controlled fake
impl SecurityCoordinatorInterface for CantripSecurityCoordinatorInterface {
    fn request(
        &mut self,
        request_id: SecurityRequest,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> Result<(), SecurityRequestError> {
        fn serialize_failure(e: postcard::Error) -> SecurityRequestError {
            trace!("serialize failed: {:?}", e);
            SecurityRequestError::SreBundleDataInvalid
        }
        fn deserialize_failure(e: postcard::Error) -> SecurityRequestError {
            trace!("deserialize failed: {:?}", e);
            SecurityRequestError::SreBundleDataInvalid
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
                        &0u32,  // TODO(sleffler): fill-in
                        reply_buffer
                ).map_err(serialize_failure)?;
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
                        ",  // TODO(sleffler): fill-in
                        reply_buffer
                ).map_err(serialize_failure)?;
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
            SecurityRequest::SrReadKey => Err(SecurityRequestError::SreReadFailed),
            SecurityRequest::SrWriteKey => Err(SecurityRequestError::SreWriteFailed),
            SecurityRequest::SrDeleteKey => Err(SecurityRequestError::SreDeleteFailed),
        }
    }
}
