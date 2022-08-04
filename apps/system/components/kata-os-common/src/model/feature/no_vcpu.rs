// No Hypervisor Support.

use crate::CantripOsModel;
use capdl::*;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Result;

use static_assertions::assert_cfg;
assert_cfg!(not(any(
    feature = "CONFIG_ARM_HYPERVISOR_SUPPORT",
    feature = "CONFIG_VTX"
)));

impl<'a> CantripOsModel<'a> {
    pub fn set_tcb_vcpu(&self, _cdl_tcb: &CDL_Object, _sel4_tcb: seL4_CPtr) -> seL4_Result {
        Ok(())
    }
}
