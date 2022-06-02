#![no_std]
#![allow(dead_code)]
use cstr_core::CString;
use cantrip_memory_interface::ObjDescBundle;

/// The Vector Core uses a Windowed MMU (go/sparrow-wmmu) in order to prevent
/// models from interferring with each other. Before executing a model,
/// windows to only that model's code and data are opened.
/// A window is represented by an address and size of that window.
pub struct Window {
    pub addr: usize,
    pub size: usize,
}

/// When a model is loaded onto the Vector Core, the ML Coordinator needs to
/// track where each window is.
pub struct ModelSections {
    pub instructions: Window,
    pub data: Window,
}

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

/// Abstraction layer over the hardware vector core.
pub trait MlCoreInterface {
    fn set_wmmu(&mut self, sections: &ModelSections);
    fn enable_interrupts(&mut self, enabled: bool);
    fn run(&mut self);
    fn load_image(&mut self, frames: &ObjDescBundle) -> Result<ModelSections, &'static str>;
    fn get_return_code() -> u32;
    fn get_fault_register() -> u32;
    fn clear_host_req();
    fn clear_finish();
    fn clear_instruction_fault();
    fn clear_data_fault();
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
