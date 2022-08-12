// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
