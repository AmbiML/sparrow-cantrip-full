#![no_std]
#![allow(dead_code)]
use cstr_core::CString;

/// Errors that can occur when interacting with the MlCoordinator.
#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum MlCoordError {
    MlCoordOk,
    InvalidModelId,
    InvalidBundleId,
    LoadModelFailed,
    NoModelSlotsLeft,
    NoSuchModel,
}

impl From<MlCoordError> for Result<(), MlCoordError> {
    fn from(err: MlCoordError) -> Result<(), MlCoordError> {
        if err == MlCoordError::MlCoordOk {
            Ok(())
        } else {
            Err(err)
        }
    }
}

#[inline]
pub fn cantrip_mlcoord_oneshot(bundle_id: &str, model_id: &str) -> Result<(), MlCoordError> {
    extern "C" {
        // NB: this assumes the MlCoordinator component is named "mlcoord".
        fn mlcoord_oneshot(
            c_bundle_id: *const cstr_core::c_char,
            c_model_id: *const cstr_core::c_char,
        ) -> MlCoordError;
    }
    let bundle_id_cstr = CString::new(bundle_id).map_err(|_| MlCoordError::InvalidBundleId)?;
    let model_id_cstr = CString::new(model_id).map_err(|_| MlCoordError::InvalidModelId)?;

    unsafe { mlcoord_oneshot(bundle_id_cstr.as_ptr(), model_id_cstr.as_ptr()) }.into()
}

#[inline]
pub fn cantrip_mlcoord_periodic(
    bundle_id: &str,
    model_id: &str,
    rate_in_ms: u32,
) -> Result<(), MlCoordError> {
    extern "C" {
        fn mlcoord_periodic(
            c_bundle_id: *const cstr_core::c_char,
            c_model_id: *const cstr_core::c_char,
            rate_in_ms: u32,
        ) -> MlCoordError;
    }
    let bundle_id_cstr = CString::new(bundle_id).map_err(|_| MlCoordError::InvalidBundleId)?;
    let model_id_cstr = CString::new(model_id).map_err(|_| MlCoordError::InvalidModelId)?;

    unsafe { mlcoord_periodic(bundle_id_cstr.as_ptr(), model_id_cstr.as_ptr(), rate_in_ms) }.into()
}

#[inline]
pub fn cantrip_mlcoord_cancel(bundle_id: &str, model_id: &str) -> Result<(), MlCoordError> {
    extern "C" {
        fn mlcoord_cancel(
            c_bundle_id: *const cstr_core::c_char,
            c_model_id: *const cstr_core::c_char,
        ) -> MlCoordError;
    }
    let bundle_id_cstr = CString::new(bundle_id).map_err(|_| MlCoordError::InvalidBundleId)?;
    let model_id_cstr = CString::new(model_id).map_err(|_| MlCoordError::InvalidModelId)?;

    unsafe { mlcoord_cancel(bundle_id_cstr.as_ptr(), model_id_cstr.as_ptr()) }.into()
}

#[inline]
pub fn cantrip_mlcoord_debug_state() {
    extern "C" {
        fn mlcoord_debug_state();
    }
    unsafe { mlcoord_debug_state() };
}
