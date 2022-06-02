#![no_std]

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

pub trait MlCoordinatorInterface {
    fn execute(&mut self, bundle_id: &str, model_id: &str);
    fn set_continuous_mode(&mut self, mode: bool);
}

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
#[allow(dead_code)]
pub fn cantrip_mlcoord_execute(bundle_id: &str, model_id: &str)
    -> Result<(),cstr_core:: NulError>
{
    extern "C" {
        // NB: this assumes the MlCoordinator component is named "mlcoord".
        fn mlcoord_execute(
            c_bundle_id: *const cstr_core::c_char,
            c_model_id: *const cstr_core::c_char
        );
    }
    let bundle_id_cstr = CString::new(bundle_id)?;
    let model_id_cstr = CString::new(model_id)?;
    unsafe { mlcoord_execute(bundle_id_cstr.as_ptr(), model_id_cstr.as_ptr()) };
    Ok(())
}
