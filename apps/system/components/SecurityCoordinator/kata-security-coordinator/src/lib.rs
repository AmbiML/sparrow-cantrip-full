//! Cantrip OS security coordinator support

#![cfg_attr(not(test), no_std)]
// NB: "error[E0658]: trait bounds other than `Sized` on const fn parameters are unstable"
#![feature(const_fn_trait_bound)]

extern crate alloc;
use alloc::boxed::Box;
use cantrip_security_interface::SecurityCoordinatorInterface;
use cantrip_security_interface::SecurityRequest;
use cantrip_security_interface::SecurityRequestError;

#[cfg(all(feature = "fake", feature = "sel4"))]
compile_error!("features \"fake\" and \"sel4\" are mutually exclusive");

#[cfg_attr(feature = "sel4", path = "impl.rs")]
#[cfg_attr(feature = "fake", path = "fakeimpl/mod.rs")]
mod platform;
pub use platform::CantripSecurityCoordinatorInterface;

#[cfg(not(test))]
pub static mut CANTRIP_SECURITY: CantripSecurityCoordinator = CantripSecurityCoordinator::empty();

// CantripSecurityCoordinator bundles an instance of the SecurityCoordinator that operates
// on CantripOS interfaces. There is a two-step dance to setup an instance because we want
// CANTRIP_STORAGE static.
// NB: no locking is done; we assume the caller/user is single-threaded
pub struct CantripSecurityCoordinator {
    manager: Option<Box<dyn SecurityCoordinatorInterface + Sync>>,
}
impl CantripSecurityCoordinator {
    // Constructs a partially-initialized instance; to complete call init().
    // This is needed because we need a const fn for static setup.
    const fn empty() -> CantripSecurityCoordinator {
        CantripSecurityCoordinator { manager: None }
    }

    pub fn init(&mut self) {
        self.manager = Some(Box::new(CantripSecurityCoordinatorInterface::new()));
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
