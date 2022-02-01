// Hypervisor Support.

use crate::CantripOsModel;
use capdl::*;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Result;

use static_assertions::assert_cfg;
assert_cfg!(any(feature = "CONFIG_ARM_HYPERVISOR_SUPPORT", feature = "CONFIG_VTX"));

impl<'a> CantripOsModel<'a> {
    pub fn set_tcb_vcpu(&self, cdl_tcb: &CDL_Object, sel4_tcb: seL4_CPtr) -> seL4_Result {
        let cap_to_cptr = |opt: Option<&CDL_Cap>| -> seL4_CPtr {
            match opt {
                Some(obj) => self.get_orig_cap(obj.obj_id),
                _ => 0,
            }
        };

        let cdl_vcpu_opt = cdl_tcb.get_cap_at(CDL_TCB_VCPU_Slot);
        let sel4_vcpu = cap_to_cptr(cdl_vcpu_opt);
        if sel4_vcpu != 0 {
            // TODO(sleffler): maybe belongs in arch support
            #[cfg(feature = "CONFIG_ARM_HYPERVISOR_SUPPORT")]
            unsafe { sel4_sys::seL4_ARM_VCPU_SetTCB(sel4_vcpu, sel4_tcb) }?;

            #[cfg(feature = "CONFIG_VTX")]
            unsafe { sel4_sys::seL4_X86_VCPU_SetTCB(sel4_vcpu, sel4_tcb) }?;
        }
        Ok(())
    }
}
