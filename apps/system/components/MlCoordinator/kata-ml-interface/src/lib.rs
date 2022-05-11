#![no_std]

use cstr_core::CString;
use cantrip_memory_interface::ObjDescBundle;

pub trait MlCoordinatorInterface {
    fn execute(&mut self, bundle_id: &str, model_id: &str);
    fn set_continuous_mode(&mut self, mode: bool);
}

pub trait MlCoreInterface {
    fn enable_interrupts(&mut self, enabled: bool);
    fn run(&mut self);
    fn load_image(&mut self, frames: &ObjDescBundle) -> Result<(), &'static str>;
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
